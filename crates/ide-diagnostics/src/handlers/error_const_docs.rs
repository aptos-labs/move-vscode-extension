use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::{RootDatabase, Severity};
use lang::Semantics;
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

    let const_item = resolve_to_local_const_item(&ctx.sema, &error_path_expr)?;
    let const_ident = const_item.value.name()?.ident_token();

    if const_item.value.outer_doc_comments(const_ident).is_empty() {
        acc.push(Diagnostic::new(
            DiagnosticCode::Lsp("missing-doc-comment-on-error-const", Severity::Warning),
            "Const is used as abort error and requires a documentation comment",
            const_item.file_range(),
        ))
    }

    Some(())
}

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn error_const_in_abort(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    abort_expr: InFile<ast::AbortExpr>,
) -> Option<()> {
    let error_path_expr = abort_expr.and_then(|it| it.error_expr().and_then(|it| it.path_expr()))?;

    let const_item = resolve_to_local_const_item(&ctx.sema, &error_path_expr)?;
    let const_ident = const_item.value.name()?.ident_token();

    if const_item.value.outer_doc_comments(const_ident).is_empty() {
        acc.push(Diagnostic::new(
            DiagnosticCode::Lsp("missing-const-doc-comment", Severity::Warning),
            "Const is used as abort error and requires a documentation comment",
            const_item.file_range(),
        ))
    }

    Some(())
}

fn resolve_to_local_const_item(
    sema: &Semantics<'_, RootDatabase>,
    error_path_expr: &InFile<ast::PathExpr>,
) -> Option<InFile<ast::Const>> {
    let (file_id, error_path_expr) = error_path_expr.unpack_ref();
    let error_path = error_path_expr.path();
    if !error_path.is_local() {
        return None;
    }
    let const_item = sema.resolve_to_element::<ast::Const>(error_path.in_file(file_id))?;
    Some(const_item)
}
