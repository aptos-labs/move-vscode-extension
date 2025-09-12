// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::hir_db::get_modules_in_file;
use crate::nameres::address::Address;
use crate::nameres::blocks::get_entries_in_block;
use crate::nameres::namespaces::{Ns, NsSet};
use crate::nameres::path_resolution::ResolutionContext;
use crate::nameres::resolve_scopes;
use crate::nameres::scope::{NamedItemsInFileExt, ScopeEntry};
use crate::nameres::scope_entries_owner::get_entries_in_scope;
use crate::{hir_db, nameres};
use base_db::SourceDatabase;
use base_db::package_root::PackageId;
use std::collections::HashSet;
use syntax::SyntaxKind;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::InFile;
use syntax::{AstNode, SyntaxNode, ast};

pub fn get_entries_from_walking_scopes(
    db: &dyn SourceDatabase,
    start_at: InFile<SyntaxNode>,
    ns: NsSet,
) -> Vec<ScopeEntry> {
    let _p = tracing::debug_span!("get_entries_from_walking_scopes").entered();

    let resolve_scopes = resolve_scopes::get_resolve_scopes(db, &start_at);
    let start_at = &start_at.value;
    let start_at_offset = start_at.text_range().start();

    let mut visited_names: HashSet<(Ns, &str)> = HashSet::new();
    let mut entries = vec![];

    for resolve_scope in resolve_scopes {
        if let Some(match_arm) = resolve_scope.value.cast::<ast::MatchArm>()
            && match_arm
                .pat()
                .is_some_and(|it| it.syntax().text_range().contains(start_at_offset))
        {
            continue;
        }

        let block_entries = resolve_scope
            .syntax_cast::<ast::BlockExpr>()
            .map(|block_expr| get_entries_in_block(db, block_expr, start_at))
            .unwrap_or_default();
        let resolve_scope_entries = get_entries_in_scope(db, &resolve_scope);

        let scope_entries_len = block_entries.len() + resolve_scope_entries.len();
        if scope_entries_len == 0 {
            continue;
        }

        let prev_visited_names = visited_names.clone();
        entries.reserve(scope_entries_len);
        visited_names.reserve(scope_entries_len);

        let scope_entries = block_entries.into_iter().chain(resolve_scope_entries.iter());
        for scope_entry in scope_entries {
            let entry_ns = scope_entry.ns;
            if !ns.contains(entry_ns) {
                continue;
            }

            let ns_pair = (entry_ns, scope_entry.name.as_str());
            if prev_visited_names.contains(&ns_pair) {
                continue;
            }
            visited_names.insert(ns_pair);

            entries.push(scope_entry.clone());
        }
    }
    entries
}

pub fn get_modules_as_entries(
    db: &dyn SourceDatabase,
    package_id: PackageId,
    address: Address,
) -> Vec<ScopeEntry> {
    let _p = tracing::debug_span!("get_modules_as_entries").entered();

    let interesting_file_ids = hir_db::file_ids_by_module_address(db, package_id, address.clone());
    tracing::debug!(?interesting_file_ids);

    let mut module_entries = Vec::with_capacity(interesting_file_ids.len());
    for source_file_id in interesting_file_ids {
        let modules = get_modules_in_file(db, source_file_id, address.clone());
        module_entries.extend(modules.to_entries(source_file_id));
    }
    tracing::debug!(?module_entries);

    module_entries
}

#[tracing::instrument(
    level = "debug",
    skip(db, ctx, qualifier),
    fields(qualifier = ?qualifier.syntax().text(), path = ?ctx.start_at.value.text()))]
pub fn get_qualified_path_entries(
    db: &dyn SourceDatabase,
    ctx: &ResolutionContext,
    qualifier: ast::Path,
) -> Vec<ScopeEntry> {
    let qualifier = ctx.wrap_in_file(qualifier);
    let qualifier_resolved = nameres::resolve_no_inf(db, qualifier.clone());
    if qualifier_resolved.is_none() {
        if let Some(qualifier_name) = qualifier.value.reference_name() {
            let _p = tracing::debug_span!(
                "qualifier is unresolved",
                "try to resolve assuming that {:?} is a named address",
                qualifier_name
            )
            .entered();
            return get_modules_as_entries(db, ctx.package_id(db), Address::named(&qualifier_name));
        }
        return vec![];
    }
    let qualifier_resolved = qualifier_resolved.unwrap();

    let mut entries = vec![];
    match qualifier_resolved.node_loc.kind() {
        SyntaxKind::MODULE => {
            let module_loc = &qualifier_resolved.node_loc;
            // Self::call() as an expression
            if ctx.is_use_speck() {
                entries.push(ScopeEntry {
                    name: "Self".to_string(),
                    node_loc: module_loc.clone(),
                    ns: Ns::MODULE,
                    scope_adjustment: None,
                });
            }
            entries.extend(hir_db::module_importable_entries(db, module_loc.clone()));
            entries.extend(hir_db::module_importable_entries_from_related(
                db,
                module_loc.clone(),
            ));
        }
        SyntaxKind::ENUM => {
            let Some(enum_) = qualifier_resolved.node_loc.to_ast::<ast::Enum>(db) else {
                return vec![];
            };
            entries.extend(enum_.value.variants().to_entries(enum_.file_id));
        }
        _ => {}
    }
    entries
}
