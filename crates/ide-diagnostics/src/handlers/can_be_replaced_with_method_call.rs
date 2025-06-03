use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use ide_db::assist_context::Assists;
use ide_db::assists::AssistId;
use ide_db::label::Label;
use lang::types::fold::TypeFoldable;
use lang::types::has_type_params_ext::GenericItemExt;
use lang::types::inference::TyVarIndex;
use lang::types::substitution::ApplySubstitution;
use syntax::SyntaxKind::BORROW_EXPR;
use syntax::ast::ReferenceElement;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::ast::syntax_factory::SyntaxFactory;
use syntax::files::{FileRange, InFile, InFileExt};
use syntax::{AstNode, ast};

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn can_be_replaced_with_method_call(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    call_expr: InFile<ast::CallExpr>,
) -> Option<Diagnostic> {
    let msl = call_expr.value.syntax().is_msl_context();

    let reference = call_expr
        .clone()
        .and_then(|it| it.path())?
        .map(|it| it.reference());
    let fun = ctx.sema.resolve_to_element::<ast::Fun>(reference)?;

    let self_param = fun.value.self_param()?;
    let self_param_type = self_param.type_()?;

    let first_arg_expr = call_expr.value.arg_exprs().first()?.to_owned()?;
    let first_arg_ty = ctx
        .sema
        .get_expr_type(&first_arg_expr.in_file(call_expr.file_id), false)?;

    // if function module is different to the first argument expr module,
    // then it's not a method even if `self` argument is present
    let fun_module = ctx.sema.fun_module(fun.clone().map_into())?.value;
    let arg_ty_module = ctx.sema.ty_module(&first_arg_ty)?;
    if fun_module != arg_ty_module {
        return None;
    }

    let fun_subst = fun.ty_vars_subst(&TyVarIndex::default());
    let self_ty = ctx
        .sema
        .lower_type(self_param_type.in_file(fun.file_id), msl)
        .substitute(&fun_subst);

    if self_ty.has_ty_unknown() || first_arg_ty.has_ty_unknown() {
        return None;
    }

    if ctx.sema.is_tys_compatible(first_arg_ty, self_ty, true) {
        acc.push(
            Diagnostic::new(
                DiagnosticCode::Lsp("replace-with-method-call", Severity::WeakWarning),
                "Can be replaced with method call",
                call_expr.file_range(),
            )
            .with_fixes(fixes(ctx, call_expr.clone(), call_expr.file_range())),
        )
    }

    None
}

#[tracing::instrument(level = "trace", skip_all)]
fn fixes(
    ctx: &DiagnosticsContext<'_>,
    call_expr: InFile<ast::CallExpr>,
    diagnostic_range: FileRange,
) -> Option<Assists> {
    let (file_id, call_expr) = call_expr.unpack();

    let call_expr_parent = call_expr.syntax().parent()?;

    let mut receiver_expr = call_expr.arg_exprs().first()?.to_owned()?;
    if receiver_expr.syntax().kind() == BORROW_EXPR {
        receiver_expr = receiver_expr.borrow_expr().unwrap().expr()?;
    }

    let name = call_expr.path()?.reference_name()?;
    let type_arg_list = call_expr.path()?.segment().and_then(|it| it.type_arg_list());

    let method_args = call_expr
        .arg_exprs()
        .clone()
        .into_iter()
        .skip(1)
        .collect::<Vec<_>>();
    if method_args.iter().any(|it| it.is_none()) {
        return None;
    }
    let method_args = method_args.into_iter().map(|it| it.unwrap()).collect::<Vec<_>>();

    let mut assists = Assists::new(file_id, ctx.resolve.clone());
    assists.add(
        AssistId::quick_fix("replace-with-method-call"),
        Label::new("Replace with method call".to_string()),
        diagnostic_range.range,
        |builder| {
            use syntax::SyntaxKind::*;

            let make = SyntaxFactory::new();
            match receiver_expr.syntax().kind() {
                // all AtomExpr list, same priority as MvDotExpr
                VECTOR_LIT_EXPR | STRUCT_LIT | TUPLE_EXPR | PAREN_EXPR | ANNOTATED_EXPR | DOT_EXPR
                | METHOD_CALL_EXPR | INDEX_EXPR | CALL_EXPR | ASSERT_MACRO_EXPR | PATH_EXPR
                | LAMBDA_EXPR | LITERAL | BLOCK_EXPR => {
                    // do nothing, those operations priorities are correct without parens
                }
                _ => {
                    receiver_expr = make.expr_paren(receiver_expr);
                }
            }
            let method_arg_list = make.arg_list(method_args);
            let method_call_expr = make.expr_method_call(
                receiver_expr,
                make.name_ref(&name),
                type_arg_list,
                method_arg_list,
            );

            let mut editor = builder.make_editor(&call_expr_parent);
            editor.replace(call_expr.syntax(), method_call_expr.syntax().clone_for_update());

            builder.add_file_edits(file_id, editor);
        },
    );

    Some(assists)
}
