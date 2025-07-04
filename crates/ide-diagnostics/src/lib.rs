// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

#![allow(dead_code)]

pub mod config;
pub mod diagnostic;
pub mod handlers;
mod tests;

use crate::config::DiagnosticsConfig;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use base_db::inputs::InternFileId;
use base_db::source_db;
use ide_db::RootDatabase;
use ide_db::assists::AssistResolveStrategy;
use lang::Semantics;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::{FileRange, InFileExt};
use syntax::{AstNode, ast, match_ast};
use vfs::FileId;

struct DiagnosticsContext<'a> {
    config: &'a DiagnosticsConfig,
    sema: Semantics<'a, RootDatabase>,
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
    source_db::parse_errors(db, file_id.intern(db))
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
    frange: FileRange,
) -> Vec<Diagnostic> {
    let _p = tracing::info_span!("semantic_diagnostics").entered();

    let FileRange { file_id, range: diag_range } = frange;
    let sema = Semantics::new(db, file_id);

    let mut acc = vec![];

    let file = sema.parse(file_id);
    let ctx = DiagnosticsContext { config, sema, resolve };
    for node in file.syntax().descendants() {
        // do not overlap
        if node.text_range().intersect(diag_range).is_none() {
            continue;
        }
        if let Some(ctx_owner) = node.clone().cast::<ast::InferenceCtxOwner>() {
            handlers::type_check(&mut acc, &ctx, &ctx_owner.in_file(file_id));
        }
        if let Some(reference_element) = node.clone().cast::<ast::ReferenceElement>() {
            handlers::find_unresolved_references(&mut acc, &ctx, reference_element.in_file(file_id));
        }
        if let Some(any_call_expr) = node.clone().cast::<ast::AnyCallExpr>() {
            handlers::call_params::check_value_arguments(&mut acc, &ctx, any_call_expr.in_file(file_id));
        }
        match_ast! {
            match node {
                ast::CallExpr(it) => {
                    handlers::can_be_replaced_with_method_call(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::IdentPat(it) => {
                    handlers::unused_variables::check_unused_ident_pat(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::BinExpr(it) => {
                    handlers::can_be_replaced_with_compound_expr(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::NamedField(it) => {
                    handlers::recursive_struct_check(&mut acc, &ctx, it.in_file(file_id));
                },
                _ => (),
            }
        }
    }

    acc
}

pub fn full_diagnostics(
    db: &RootDatabase,
    config: &DiagnosticsConfig,
    resolve: &AssistResolveStrategy,
    file_range: FileRange,
) -> Vec<Diagnostic> {
    let mut res = vec![];
    res.extend(syntax_diagnostics(db, config, file_range.file_id));
    res.extend(semantic_diagnostics(db, config, resolve, file_range));
    res
}
