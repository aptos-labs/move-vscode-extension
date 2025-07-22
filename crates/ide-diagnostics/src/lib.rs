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
use ide_db::assist_context::Assists;
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

impl DiagnosticsContext<'_> {
    pub fn assists_for_file(&self, file_id: FileId) -> Assists {
        Assists::new(file_id, self.resolve.clone())
    }
}

/// Request parser level diagnostics for the given [`FileId`].
pub fn syntax_diagnostics(
    db: &RootDatabase,
    config: &DiagnosticsConfig,
    file_id: FileId,
) -> Vec<Diagnostic> {
    let _p = tracing::info_span!("syntax_diagnostics").entered();

    if !config.is_diagnostic_enabled("syntax_error") {
        // if config.disabled.contains("syntax-error") {
        return Vec::new();
    }

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
        if let Some(method_or_path) = node.clone().cast::<ast::MethodOrPath>() {
            handlers::missing_type_arguments::missing_type_arguments(
                &mut acc,
                &ctx,
                method_or_path.in_file(file_id),
            );
        }
        match_ast! {
            match node {
                ast::CallExpr(it) => {
                    handlers::can_be_replaced_with_method_call(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::AssertMacroExpr(it) => {
                    handlers::error_const_docs::error_const_in_assert(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::AbortExpr(it) => {
                    handlers::error_const_docs::error_const_in_abort(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::IdentPat(it) => {
                    handlers::unused_variables::check_unused_ident_pat(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::BinExpr(it) => {
                    handlers::can_be_replaced_with_compound_expr(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::DerefExpr(it) => {
                    handlers::can_be_replaced_with_index_expr(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::NamedField(it) => {
                    handlers::recursive_struct_check(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::StructLit(it) => {
                    handlers::missing_fields::missing_fields_in_struct_lit(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::StructLitField(it) => {
                    handlers::field_shorthand::struct_lit_field_can_be_simplified(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::StructPatField(it) => {
                    handlers::field_shorthand::struct_pat_field_can_be_simplified(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::SchemaLitField(it) => {
                    handlers::field_shorthand::schema_lit_field_can_be_simplified(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::StructPat(it) => {
                    handlers::missing_fields::missing_fields_in_struct_pat(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::TupleStructPat(it) => {
                    handlers::missing_fields::missing_fields_in_tuple_struct_pat(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::CastExpr(it) => {
                    handlers::redundant_integer_cast(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::SpecFun(it) => {
                    handlers::check_syntax::spec_fun_requires_return_type(&mut acc, &ctx, it.in_file(file_id));
                },
                ast::Fun(it) => {
                    let fun = it.in_file(file_id);
                    handlers::check_syntax::entry_fun_cannot_have_return_type(&mut acc, &ctx, fun.clone());
                    handlers::unused_acquires::unused_acquires_on_inline_function(&mut acc, &ctx, fun.clone());
                },
                _ => (),
            }
        }
    }

    acc.retain(|d| ctx.config.is_diagnostic_enabled(d.code.as_str()));
    // acc.retain(|d| !ctx.config.disabled.contains(d.code.as_str()));

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
