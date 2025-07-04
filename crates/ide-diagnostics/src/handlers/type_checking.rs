// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use lang::types::fold::TypeFoldable;
use lang::types::inference::TypeError;
use lang::types::ty::Ty;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{FileRange, InFile, InFileExt};
use syntax::{AstNode, ast};
use vfs::FileId;

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn recursive_struct_check(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    named_field: InFile<ast::NamedField>,
) -> Option<()> {
    if !ctx.config.type_checking_enabled {
        return None;
    }
    if ctx.config.assists_only {
        return None;
    }
    let (file_id, named_field) = named_field.unpack();
    let field_type = named_field.type_()?;
    let leaf_path_types = field_type.syntax().descendants_of_type::<ast::PathType>();

    let owner_item = named_field.fields_owner().struct_or_enum().in_file(file_id);
    let owner_item_name = owner_item.value.name()?.as_string();
    for leaf_path_type in leaf_path_types {
        let leaf_path = leaf_path_type.path().in_file(file_id);
        let item = ctx
            .sema
            .resolve_to_element::<ast::StructOrEnum>(leaf_path.clone().map_into());
        if let Some(item) = item {
            if item == owner_item {
                register_type_error(
                    acc,
                    ctx,
                    file_id,
                    &TypeError::circular_type(leaf_path.value, owner_item_name.clone()),
                );
            }
        }
    }

    Some(())
}

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn type_check(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    inference_ctx_owner: &InFile<ast::InferenceCtxOwner>,
) -> Option<()> {
    if !ctx.config.type_checking_enabled {
        return None;
    }
    if ctx.config.assists_only {
        // no assists for type checking
        return None;
    }
    let msl = inference_ctx_owner.value.syntax().is_msl_context();
    let inference = ctx.sema.inference(inference_ctx_owner, msl)?;
    let file_id = inference_ctx_owner.file_id;

    let mut remaining_errors = inference.type_errors.clone();
    // drop all type errors with ty unknown inside to prevent false positives
    remaining_errors.retain(|type_error| !type_error.has_ty_unknown());

    remaining_errors.sort_by_key(|err| err.text_range().start());
    // need to reverse() to pop() correctly
    remaining_errors.reverse();
    'outer: while let Some(type_error) = remaining_errors.pop() {
        // if any of the remaining errors are inside the range, then drop the error
        let error_range = type_error.text_range();
        for remaining_error in remaining_errors.iter() {
            if error_range.contains_range(remaining_error.text_range()) {
                continue 'outer;
            }
        }
        register_type_error(acc, ctx, file_id, &type_error);
    }

    Some(())
}

fn register_type_error(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    file_id: FileId,
    type_error: &TypeError,
) {
    match type_error {
        TypeError::TypeMismatch {
            text_range,
            actual_ty,
            expected_ty,
        } => {
            let actual = ctx.sema.render_ty(actual_ty);
            let expected = ctx.sema.render_ty(expected_ty);
            acc.push(Diagnostic::new(
                DiagnosticCode::Lsp("type-error", Severity::Error),
                format!("Incompatible type '{actual}', expected '{expected}'"),
                FileRange { file_id, range: *text_range },
            ))
        }
        TypeError::UnsupportedOp { text_range, ty, op } => {
            let ty = ctx.sema.render_ty(ty);
            acc.push(Diagnostic::new(
                DiagnosticCode::Lsp("type-error", Severity::Error),
                format!("Invalid argument to '{op}': expected integer type, but found '{ty}'"),
                FileRange { file_id, range: *text_range },
            ))
        }
        TypeError::WrongArgumentsToBinExpr {
            text_range,
            left_ty,
            right_ty,
            op,
        } => {
            let left_ty = ctx.sema.render_ty(left_ty);
            let right_ty = ctx.sema.render_ty(right_ty);
            acc.push(Diagnostic::new(
                DiagnosticCode::Lsp("type-error", Severity::Error),
                format!("Incompatible arguments to '{op}': '{left_ty}' and '{right_ty}'"),
                FileRange { file_id, range: *text_range },
            ))
        }
        TypeError::InvalidUnpacking {
            text_range,
            pat_kind: kind,
            assigned_ty,
        } => {
            use syntax::SyntaxKind::*;

            let message = match kind {
                STRUCT_PAT if !matches!(assigned_ty, Ty::Adt(_) | Ty::Tuple(_)) => {
                    format!(
                        "Assigned expr of type '{}' cannot be unpacked with struct pattern",
                        ctx.sema.render_ty(assigned_ty)
                    )
                }
                TUPLE_PAT if !matches!(assigned_ty, Ty::Adt(_) | Ty::Tuple(_)) => {
                    format!(
                        "Assigned expr of type '{}' cannot be unpacked with tuple pattern",
                        ctx.sema.render_ty(assigned_ty)
                    )
                }
                _ => {
                    format!(
                        "Invalid unpacking. Expected {}",
                        ctx.sema.render_ty_expected_form(assigned_ty)
                    )
                }
            };
            acc.push(Diagnostic::new(
                DiagnosticCode::Lsp("type-error", Severity::Error),
                message,
                FileRange { file_id, range: *text_range },
            ))
        }
        TypeError::CircularType { text_range, type_name } => acc.push(Diagnostic::new(
            DiagnosticCode::Lsp("type-error", Severity::Error),
            format!("Circular reference of type '{type_name}'"),
            FileRange { file_id, range: *text_range },
        )),
        TypeError::WrongArgumentToBorrowExpr { text_range, actual_ty } => {
            let ty = ctx.sema.render_ty(actual_ty);
            acc.push(Diagnostic::new(
                DiagnosticCode::Lsp("type-error", Severity::Error),
                format!("Expected a single non-reference type, but found '{ty}'"),
                FileRange { file_id, range: *text_range },
            ))
        }
        TypeError::InvalidDereference { text_range, actual_ty } => {
            let ty = ctx.sema.render_ty(actual_ty);
            acc.push(Diagnostic::new(
                DiagnosticCode::Lsp("type-error", Severity::Error),
                format!("Invalid dereference. Expected '&_' but found '{ty}'"),
                FileRange { file_id, range: *text_range },
            ))
        }
    }
}
