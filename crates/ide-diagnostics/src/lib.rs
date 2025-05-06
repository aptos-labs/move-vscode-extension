#![allow(dead_code)]

pub mod config;
pub mod diagnostic;
mod handlers;
mod tests;

use crate::config::DiagnosticsConfig;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use base_db::ParseDatabase;
use base_db::inputs::InternFileId;
use ide_db::RootDatabase;
use ide_db::assists::AssistResolveStrategy;
use lang::Semantics;
use syntax::files::{FileRange, InFileExt};
use syntax::{AstNode, ast, match_ast};
use vfs::FileId;

struct DiagnosticsContext<'a> {
    config: &'a DiagnosticsConfig,
    sema: Semantics<'a>,
    resolve: &'a AssistResolveStrategy,
}

/// Request parser level diagnostics for the given [`FileId`].
pub fn syntax_diagnostics(
    db: &RootDatabase,
    _config: &DiagnosticsConfig,
    file_id: FileId,
) -> Vec<Diagnostic> {
    let _p = tracing::info_span!("syntax_diagnostics").entered();

    // [#3434] Only take first 128 errors to prevent slowing down editor/ide, the number 128 is chosen arbitrarily.
    db.parse_errors(file_id.intern(db))
        .as_deref()
        .into_iter()
        .flatten()
        .take(128)
        .map(|err| {
            Diagnostic::new(
                DiagnosticCode::SyntaxError,
                format!("Syntax Error: {err}"),
                FileRange {
                    file_id: file_id.into(),
                    range: err.range(),
                },
            )
        })
        .collect()
}

pub fn semantic_diagnostics(
    db: &RootDatabase,
    config: &DiagnosticsConfig,
    resolve: &AssistResolveStrategy,
    file_id: FileId,
) -> Vec<Diagnostic> {
    let _p = tracing::info_span!("semantic_diagnostics").entered();
    let sema = Semantics::new(db, file_id);

    let mut res = vec![];

    let file = sema.parse(file_id);
    let ctx = DiagnosticsContext { config, sema, resolve };
    for node in file.syntax().descendants() {
        match_ast! {
            match node {
                ast::CallExpr(it) => {
                    handlers::can_be_replaced_with_method_call(&mut res, &ctx, it.in_file(file_id));
                },
                ast::AnyReferenceElement(it) => {
                    handlers::unresolved_reference(&mut res, &ctx, it.in_file(file_id));
                },
                ast::BinExpr(it) => {
                    handlers::can_be_replaced_with_compound_expr(&mut res, &ctx, it.in_file(file_id));
                },
                _ => (),
            }
        }
    }

    res
}

pub fn full_diagnostics(
    db: &RootDatabase,
    config: &DiagnosticsConfig,
    resolve: &AssistResolveStrategy,
    file_id: FileId,
) -> Vec<Diagnostic> {
    let mut res = syntax_diagnostics(db, config, file_id);
    res.extend(semantic_diagnostics(db, config, resolve, file_id));
    res
}
