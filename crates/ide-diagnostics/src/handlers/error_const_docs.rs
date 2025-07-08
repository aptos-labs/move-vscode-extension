use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use syntax::ast;
use syntax::ast::HoverDocsOwner;
use syntax::files::{InFile, InFileExt};

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn error_const_in_assert(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    assert_expr: InFile<ast::AssertMacroExpr>,
) -> Option<()> {
    let error_path_expr = assert_expr.and_then(|it| it.error_expr().and_then(|it| it.path_expr()))?;

    check_if_resolved_to_documented_error_const(acc, ctx, error_path_expr)?;

    Some(())
}

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn error_const_in_abort(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    abort_expr: InFile<ast::AbortExpr>,
) -> Option<()> {
    let error_path_expr = abort_expr.and_then(|it| it.error_expr().and_then(|it| it.path_expr()))?;

    check_if_resolved_to_documented_error_const(acc, ctx, error_path_expr)?;

    Some(())
}

fn check_if_resolved_to_documented_error_const(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    error_path_expr: InFile<ast::PathExpr>,
) -> Option<()> {
    let (file_id, error_path_expr) = error_path_expr.unpack();
    let error_path = error_path_expr.path();
    if !error_path.is_local() {
        return None;
    }
    let const_item = ctx
        .sema
        .resolve_to_element::<ast::Const>(error_path.in_file(file_id))?;

    let const_ident = const_item.clone().and_then(|it| it.name())?;

    if const_item
        .value
        .outer_doc_comments(const_ident.value.ident_token())
        .is_empty()
    {
        acc.push(Diagnostic::new(
            DiagnosticCode::Lsp("missing-const-doc-comment", Severity::Warning),
            "Missing documentation comment (provides a human-readable error message on-chain)",
            const_ident.file_range(),
        ))
    }

    Some(())
}
