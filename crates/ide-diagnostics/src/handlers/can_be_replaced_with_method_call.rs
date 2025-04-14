use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use ide_db::assists::{Assist, AssistId};
use ide_db::label::Label;
use ide_db::source_change::SourceChangeBuilder;
use lang::db::ExprInferenceExt;
use lang::nameres::ResolveReference;
use lang::types::has_type_params_ext::GenericItemExt;
use lang::types::inference::InferenceCtx;
use lang::types::lowering::TyLowering;
use lang::types::substitution::ApplySubstitution;
use syntax::ast::syntax_factory::SyntaxFactory;
use syntax::files::{FileRange, InFile, InFileExt};
use syntax::{AstNode, ast};

pub(crate) fn can_be_replaced_with_method_call(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    call_expr: InFile<ast::CallExpr>,
) -> Option<Diagnostic> {
    let file_id = call_expr.file_id;
    let db = ctx.db();
    let (fun_file_id, fun) = call_expr
        .clone()
        .map(|it| it.path().reference())
        .resolve(db)?
        .cast_into::<ast::Fun>(db)?
        .unpack();
    let self_param = fun.self_param()?;
    let self_param_type = self_param.type_()?;

    let inference = call_expr.clone().in_file_into::<ast::Expr>().inference(db)?;
    let first_arg_expr = call_expr.value.args().first()?.to_owned();
    let first_arg_expr_ty = inference.get_expr_type(&first_arg_expr)?;

    let fun_module = fun.module()?;
    let arg_ty_module = first_arg_expr_ty.inner_item_module(db, file_id)?.value;
    if fun_module != arg_ty_module {
        return None;
    }

    let fun_subst = fun.in_file(fun_file_id).ty_vars_subst();
    let self_ty = TyLowering::new_no_inf(db)
        .lower_type(self_param_type.in_file(fun_file_id))
        .substitute(&fun_subst);

    let mut inf = InferenceCtx::new(db, file_id);
    if inf.is_tys_compatible_with_autoborrow(first_arg_expr_ty, self_ty) {
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

fn fixes(
    ctx: &DiagnosticsContext<'_>,
    call_expr: InFile<ast::CallExpr>,
    diagnostic_range: FileRange,
) -> Option<Vec<Assist>> {
    use syntax::SyntaxKind::*;

    let (file_id, call_expr) = call_expr.unpack();

    let call_expr_parent = call_expr.syntax().parent()?;
    let make = SyntaxFactory::new();
    let mut builder = SourceChangeBuilder::new(file_id);

    let mut receiver_expr = call_expr.args().first()?.to_owned();
    if receiver_expr.syntax().kind() == BORROW_EXPR {
        receiver_expr = receiver_expr.borrow_expr().unwrap().expr()?;
    }

    match receiver_expr.syntax().kind() {
        // all AtomExpr list, same priority as MvDotExpr
        VECTOR_LIT_EXPR | STRUCT_LIT | TUPLE_EXPR | PAREN_EXPR | ANNOTATED_EXPR | DOT_EXPR
        | METHOD_CALL_EXPR | INDEX_EXPR | CALL_EXPR | ASSERT_MACRO_EXPR | PATH_EXPR | LAMBDA_EXPR
        | LITERAL | BLOCK_EXPR => {
            // do nothing, those operations priorities are correct without parens
        }
        _ => {
            receiver_expr = make.expr_paren(receiver_expr);
        }
    }

    let method_args = call_expr.args().clone().into_iter().skip(1).collect::<Vec<_>>();
    let method_arg_list = make.arg_list(method_args);

    let type_arg_list = call_expr.path().segment().and_then(|it| it.type_arg_list());

    let name = call_expr.path().reference_name()?;
    let method_call_expr = make.expr_method_call(
        receiver_expr,
        make.name_ref(&name),
        type_arg_list,
        method_arg_list,
    );

    let mut editor = builder.make_editor(&call_expr_parent);
    editor.replace(call_expr.syntax(), method_call_expr.syntax().clone_for_update());

    builder.add_file_edits(file_id, editor);

    let source_change = builder.finish();
    let assist = Assist {
        id: AssistId::quick_fix("replace-with-method-call"),
        label: Label::new("Replace with method call".to_string()),
        group: None,
        target: diagnostic_range.range,
        source_change: Some(source_change),
        command: None,
    };
    Some(vec![assist])
}
