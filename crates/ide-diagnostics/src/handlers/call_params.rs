use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::{FileRange, InFile, InFileExt};
use syntax::{AstNode, ast};

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn check_value_arguments<'db>(
    acc: &mut Vec<Diagnostic>,
    ctx: &'db DiagnosticsContext<'db>,
    any_call_expr: InFile<ast::AnyCallExpr>,
) -> Option<()> {
    let (file_id, any_call_expr) = any_call_expr.unpack();

    let value_arg_list = any_call_expr.value_arg_list()?;

    let mut arg_exprs = vec![];
    for value_arg in value_arg_list.args() {
        let expr = value_arg.expr()?;
        arg_exprs.push(expr);
    }

    // use range, because assert! can have either 1 or 2 arguments
    let (min, max) = match any_call_expr {
        ast::AnyCallExpr::CallExpr(call_expr) => {
            let ty_callable = ctx
                .sema
                .get_call_expr_type(&call_expr.in_file(file_id).map_into())?;
            let expected_count = ty_callable.param_types.len();
            (expected_count, expected_count)
        }
        ast::AnyCallExpr::MethodCallExpr(call_expr) => {
            let ty_callable = ctx
                .sema
                .get_call_expr_type(&call_expr.in_file(file_id).map_into())?;
            // -1 for self argument
            let expected_count = ty_callable.param_types.len() - 1;
            (expected_count, expected_count)
        }
        ast::AnyCallExpr::AssertMacroExpr(_) => (1, 2),
    };
    let actual_count = arg_exprs.len();

    let expected_count_message = if min == max {
        format!("{min}")
    } else {
        format!("{min} to {max}")
    };
    if actual_count < min {
        let range = value_arg_list
            .r_paren_token()
            .map(|it| it.text_range())
            .unwrap_or(value_arg_list.syntax().text_range());
        acc.push(Diagnostic::new(
            DiagnosticCode::Lsp("arguments-number-mismatch", Severity::Warning),
            format!("This function takes {expected_count_message} parameters, but {actual_count} parameters were supplied"),
            FileRange { file_id, range },
        ));
        return Some(());
    }

    if actual_count > max {
        for error_expr in arg_exprs.iter().skip(max) {
            let range = error_expr.syntax().text_range();
            acc.push(Diagnostic::new(
                DiagnosticCode::Lsp("arguments-number-mismatch", Severity::Warning),
                format!("This function takes {expected_count_message} parameters, but {actual_count} parameters were supplied"),
                FileRange { file_id, range },
            ));
            return Some(());
        }
    }

    Some(())
}
