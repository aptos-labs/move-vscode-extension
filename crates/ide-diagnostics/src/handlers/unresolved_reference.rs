// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

mod auto_import;

use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use base_db::SourceDatabase;
use ide_db::{RootDatabase, Severity};
use lang::nameres::path_kind::{PathKind, QualifiedKind, path_kind};
use lang::nameres::scope::{ScopeEntry, VecExt, into_field_shorthand_items};
use lang::node_ext::item_spec;
use lang::types::ty::Ty;
use lang::{Semantics, hir_db};
use std::collections::HashSet;
use syntax::ast::idents::PRIMITIVE_TYPES;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn find_unresolved_references(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    reference: InFile<ast::ReferenceElement>,
) -> Option<()> {
    let msl = reference.value.syntax().is_msl_context();
    if msl && is_special_msl_path(ctx.sema.db, reference.as_ref()).is_some() {
        return None;
    }

    if let Some(path) = reference.clone().cast_into::<ast::Path>() {
        unresolved_path(acc, ctx, path)?;
    }
    if let Some(method_or_dot) = reference.clone().cast_into::<ast::MethodOrDotExpr>() {
        unresolved_method_or_dot_expr(acc, ctx, method_or_dot)?;
    }
    if let Some(struct_lit_field) = reference.clone().cast_into::<ast::StructLitField>() {
        try_check_resolve(acc, ctx, struct_lit_field.map_into());
    }
    if let Some(struct_pat_field) = reference.clone().cast_into::<ast::StructPatField>() {
        try_check_resolve(acc, ctx, struct_pat_field.map_into());
    }

    Some(())
}

fn unresolved_path(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    reference: InFile<ast::Path>,
) -> Option<()> {
    let (file_id, path) = reference.clone().unpack();

    let root_path = path.root_path();
    if path != root_path {
        // only check root_path itself
        return None;
    }

    let path_name = path.reference_name()?;
    // (_, _) = (1, 1);
    if path_name == "_" {
        for ancestor in path.syntax().ancestors() {
            if let Some((lhs, (_, op), _)) = ancestor.cast::<ast::BinExpr>().and_then(|it| it.unpack()) {
                // check for lhs of assignment expr
                if matches!(op, ast::BinaryOp::Assignment { .. })
                    && lhs.syntax().is_ancestor_of(&path.syntax())
                {
                    return None;
                }
            }
        }
    }

    if let Some(_) = path.syntax().parent_of_type::<ast::PathType>() {
        if PRIMITIVE_TYPES.contains(&path_name.as_str()) {
            return None;
        }
    }

    if path.syntax().ancestor_strict::<ast::AttrItem>().is_some() {
        return None;
    }

    // iterate over all qualifiers, stop if there's unresolved reference
    let mut base_path = Some(root_path.base_path());
    while let Some(path) = base_path {
        let path_kind = path_kind(ctx.sema.db, path.qualifier(), &path, false)?;
        match path_kind {
            PathKind::NamedAddress(_)
            | PathKind::NamedAddressOrUnqualifiedPath { .. }
            | PathKind::ValueAddress(_) => (),
            PathKind::FieldShorthand { .. } | PathKind::Unqualified { .. } => {
                if let Some(_) = try_check_resolve(acc, ctx, path.reference().in_file(file_id)) {
                    break;
                }
            }
            PathKind::Qualified { qualifier, kind, .. } => {
                match kind {
                    QualifiedKind::ModuleItemOrEnumVariant
                    | QualifiedKind::FQModuleItem
                    | QualifiedKind::UseGroupItem => {
                        let resolved = ctx.sema.resolve(qualifier).single_or_none();
                        // qualifier is unresolved, no need to resolve current path
                        if resolved.is_none() {
                            return None;
                        }
                    }
                    _ => {}
                };
                if let Some(_) = try_check_resolve(acc, ctx, path.reference().in_file(file_id)) {
                    break;
                }
            }
        }
        base_path = path.syntax().parent_of_type::<ast::Path>();
    }

    Some(())
}

fn is_special_msl_path(
    db: &dyn SourceDatabase,
    reference: InFile<&ast::ReferenceElement>,
) -> Option<()> {
    if reference
        .value
        .syntax()
        .has_ancestor_strict::<ast::ImplyIncludeExpr>()
    {
        return Some(());
    }

    let update_field_call_expr = reference.value.syntax().ancestors().find(|it| {
        it.cast::<ast::CallExpr>()
            .is_some_and(|call_expr| call_expr.syntax().text().to_string().starts_with("update_field"))
    });
    if update_field_call_expr.is_some() {
        return Some(());
    }

    let pragma_stmt = reference.value.syntax().ancestor_strict::<ast::PragmaStmt>();
    if pragma_stmt.is_some() {
        return Some(());
    }

    let path_expr = reference.and_then(|it| it.clone().path().and_then(|it| it.path_expr()))?;

    if item_spec::infer_special_path_expr_for_item_spec(db, path_expr.as_ref()).is_some() {
        return Some(());
    }

    None
}

fn unresolved_method_or_dot_expr(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    method_or_dot_expr: InFile<ast::MethodOrDotExpr>,
) -> Option<()> {
    let receiver_expr = method_or_dot_expr.map_ref(|it| it.receiver_expr());
    let receiver_ty = ctx.sema.get_expr_type(&receiver_expr)?.unwrap_all_refs();
    if matches!(receiver_ty, Ty::Unknown) {
        // no error if receiver item is unknown (won't proceed if unknown is nested)
        return None;
    }
    let (file_id, method_or_dot_expr) = method_or_dot_expr.unpack();
    let reference: ast::ReferenceElement = match method_or_dot_expr {
        ast::MethodOrDotExpr::MethodCallExpr(method_call_expr) => method_call_expr.into(),
        ast::MethodOrDotExpr::DotExpr(dot_expr) => dot_expr.into(),
    };
    try_check_resolve(acc, ctx, reference.in_file(file_id));
    Some(())
}

fn try_check_resolve(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    reference: InFile<ast::ReferenceElement>,
) -> Option<()> {
    let entries = ctx.sema.resolve_in_file(reference.clone());

    let (file_id, reference) = reference.unpack();

    let reference_name = reference.reference_name()?;
    let reference_range = reference.reference_name_node()?.in_file(file_id).file_range();

    let fixes = reference
        .path()
        .and_then(|it| auto_import::auto_import_fix(ctx, it.in_file(file_id), reference_range));

    match entries.len() {
        0 => {
            let db = ctx.sema.db;
            let package_id = db.file_package_id(file_id);
            let package_missing_deps = hir_db::missing_dependencies(db, package_id);
            let mut error_message = format!("Unresolved reference `{}`: cannot resolve", reference_name);
            if !package_missing_deps.is_empty() {
                stdx::format_to!(
                    &mut error_message,
                    " (note: `{}` declared dependency packages are not found on the filesystem, `aptos move compile` might help)",
                    package_missing_deps.join(", "),
                );
            }
            acc.push(
                Diagnostic::new(
                    DiagnosticCode::Lsp("unresolved-reference", Severity::Error),
                    error_message,
                    reference_range,
                )
                .with_local_fixes(fixes),
            );
            return Some(());
        }
        1 => (),
        _ => {
            if into_field_shorthand_items(ctx.sema.db, entries.clone()).is_some() {
                return None;
            }
            let error_message = if is_entries_from_duplicate_dependencies(&ctx.sema, entries) {
                format!(
                    "Unresolved reference `{}`: resolved to multiple elements from different packages. \
                        You have duplicate dependencies in your package manifest.",
                    reference_name
                )
            } else {
                format!(
                    "Unresolved reference `{}`: resolved to multiple elements",
                    reference_name
                )
            };
            acc.push(
                Diagnostic::new(
                    DiagnosticCode::Lsp("unresolved-reference", Severity::Error),
                    error_message,
                    reference_range,
                )
                .with_local_fixes(fixes),
            );
            return Some(());
        }
    }
    None
}

fn is_entries_from_duplicate_dependencies(
    sema: &Semantics<'_, RootDatabase>,
    entries: Vec<ScopeEntry>,
) -> bool {
    let package_ids = entries
        .iter()
        .map(|it| sema.db.file_package_id(it.node_loc.file_id()))
        .collect::<HashSet<_>>();
    package_ids.len() > 1
}
