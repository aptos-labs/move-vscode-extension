use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::assist_context::Assists;
use ide_db::assists::AssistId;
use ide_db::label::Label;
use ide_db::{RootDatabase, Severity};
use lang::Semantics;
use lang::loc::SyntaxLocNodeExt;
use lang::nameres::address;
use lang::nameres::fq_named_element::ItemFQName;
use lang::types::abilities::Ability;
use syntax::ast::syntax_factory::SyntaxFactory;
use syntax::files::{FileRange, InFile, InFileExt};
use syntax::{AstNode, ast};

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn can_be_replaced_with_index_expr(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    outer_deref_expr: InFile<ast::DerefExpr>,
) -> Option<()> {
    let (file_id, deref_expr) = outer_deref_expr.clone().unpack();

    let inference = ctx.sema.inference(&outer_deref_expr, outer_deref_expr.is_msl())?;
    // fail if any type errors
    if inference
        .type_errors
        .iter()
        .any(|it| deref_expr.syntax().text_range().contains_range(it.text_range()))
    {
        return None;
    }

    let inner_expr = deref_expr.expr()?;

    let (first_arg_expr, any_call_expr) = match inner_expr {
        ast::Expr::CallExpr(call_expr) => {
            let path = call_expr.path()?;
            if path.ident_token()?.text() != "borrow" {
                return None;
            }
            if !is_std_vector_borrow(&ctx.sema, path.in_file(file_id)).unwrap_or(false) {
                return None;
            }
            let first_arg_expr = call_expr.arg_exprs().into_iter().next()??;
            (first_arg_expr, ast::AnyCallExpr::from(call_expr))
        }
        ast::Expr::MethodCallExpr(method_call_expr) => {
            if method_call_expr.name_ref()?.as_string() != "borrow" {
                return None;
            }
            if !is_std_vector_borrow(&ctx.sema, method_call_expr.clone().in_file(file_id))
                .unwrap_or(false)
            {
                return None;
            }
            let first_arg_expr = method_call_expr.receiver_expr();
            (first_arg_expr, ast::AnyCallExpr::from(method_call_expr))
        }
        _ => {
            return None;
        }
    };

    let vector_item_ty = inference
        .get_expr_type(&first_arg_expr.loc(file_id))?
        .unwrap_all_refs()
        .into_ty_seq()?
        .item();
    if vector_item_ty
        .abilities(ctx.sema.db)
        .is_none_or(|it| !it.contains(&Ability::Copy))
    {
        return None;
    }

    let file_range = outer_deref_expr.file_range();
    acc.push(
        Diagnostic::new(
            DiagnosticCode::Lsp("replace-with-index-expr", Severity::WeakWarning),
            "Can be replaced with index expr",
            file_range,
        )
        .with_fixes(fixes(ctx, outer_deref_expr, any_call_expr, file_range)),
    );

    Some(())
}

fn fixes(
    ctx: &DiagnosticsContext<'_>,
    deref_expr: InFile<ast::DerefExpr>,
    any_call_expr: ast::AnyCallExpr,
    diagnostic_range: FileRange,
) -> Option<Assists> {
    let (file_id, deref_expr) = deref_expr.unpack();
    let edit_root = deref_expr.syntax().parent()?;
    let (receiver_expr, arg_expr) = match any_call_expr {
        ast::AnyCallExpr::CallExpr(call_expr) => {
            let mut args = call_expr.arg_exprs().into_iter();
            let receiver_param_expr = args.next()??;
            let arg_param_expr = args.next()??;
            (receiver_param_expr, arg_param_expr)
        }
        ast::AnyCallExpr::MethodCallExpr(method_call_expr) => {
            let mut args = method_call_expr.arg_exprs().into_iter();
            let receiver_param_expr = method_call_expr.receiver_expr();
            let arg_param_expr = args.next()??;
            (receiver_param_expr, arg_param_expr)
        }
        _ => {
            return None;
        }
    };

    let mut with_parens = false;
    let base_expr = match &receiver_expr {
        ast::Expr::PathExpr(_) | ast::Expr::ParenExpr(_) => receiver_expr,
        ast::Expr::BorrowExpr(borrow_expr) => borrow_expr.expr()?,
        _ => {
            with_parens = true;
            receiver_expr
        }
    };

    let mut assists = Assists::new(file_id, ctx.resolve.clone());
    assists.add(
        AssistId::quick_fix("replace-with-index-expr"),
        Label::new("Replace with vector index expr".to_string()),
        diagnostic_range.range,
        |builder| {
            let make = SyntaxFactory::new();
            let mut editor = builder.make_editor(&edit_root);

            let mut base_expr = base_expr;
            if with_parens {
                base_expr = make.paren_expr(base_expr).into();
            }
            let new_index_expr = make.index_expr(base_expr, arg_expr);
            editor.replace(deref_expr.syntax(), new_index_expr.syntax());

            editor.add_mappings(make.finish_with_mappings());
            builder.add_file_edits(file_id, editor);
        },
    );

    Some(assists)
}

fn is_std_vector_borrow(
    sema: &Semantics<'_, RootDatabase>,
    reference: InFile<impl Into<ast::ReferenceElement>>,
) -> Option<bool> {
    let fun = sema.resolve_to_element::<ast::Fun>(reference)?;
    let fun_fq_item = sema.fq_name_for_file_item(fun)?;
    let named_std_vector_borrow =
        ItemFQName::new_item(address::Address::named("std"), "vector", "borrow");
    if fun_fq_item == named_std_vector_borrow {
        return Some(true);
    }
    let value_std_vector_borrow =
        ItemFQName::new_item(address::Address::value("0x1"), "vector", "borrow");
    if fun_fq_item == value_std_vector_borrow {
        return Some(true);
    }
    Some(false)
}
