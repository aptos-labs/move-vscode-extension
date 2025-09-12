// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::nameres::resolve_scopes::ResolveScope;
use crate::nameres::scope::{NamedItemsExt, NamedItemsInFileExt, ScopeEntry};
use stdx::itertools::Itertools;
use syntax::ast::HasStmts;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::InFile;
use syntax::{AstNode, SyntaxNode, ast};

pub fn get_entries_in_blocks(resolve_scope: &ResolveScope, start_at: &SyntaxNode) -> Vec<ScopeEntry> {
    use syntax::SyntaxKind::*;

    if let Some(block_expr) = resolve_scope.scope().syntax_cast::<ast::BlockExpr>() {
        let mut entries = vec![];
        let start_at_offset = start_at.text_range().start();

        let is_msl = block_expr.is_msl();
        if is_msl {
            let spec_inline_funs = block_expr.map_ref(|it| it.spec_inline_functions()).flatten();
            entries.extend(spec_inline_funs.to_entries());
        }

        let let_stmts = let_stmts_with_bindings(block_expr);
        let current_let_stmt = start_at.ancestor_of_type::<ast::LetStmt>(false);

        let bindings = let_stmts
            .into_iter()
            .filter(|(let_stmt, _)| {
                if !is_msl {
                    return let_stmt.syntax().strictly_before_offset(start_at_offset);
                }
                if let Some(current_let_stmt) = current_let_stmt.as_ref() {
                    let is_post_visible = current_let_stmt.is_post() || !let_stmt.is_post();
                    return is_post_visible && let_stmt.syntax().strictly_before_offset(start_at_offset);
                }
                true
            })
            .collect::<Vec<_>>();

        let binding_entries = bindings.into_iter().rev().flat_map(|(_, bindings)| bindings);

        let binding_entries_with_shadowing =
            binding_entries.unique_by(|e| e.name.clone()).collect::<Vec<_>>();

        entries.extend(binding_entries_with_shadowing);
        return entries;
    }

    vec![]
}

fn let_stmts_with_bindings(block: InFile<ast::BlockExpr>) -> Vec<(ast::LetStmt, Vec<ScopeEntry>)> {
    block
        .value
        .let_stmts()
        .map(|let_stmt| {
            let bindings = let_stmt.pat().map(|pat| pat.bindings()).unwrap_or_default();
            (let_stmt, bindings.to_entries(block.file_id))
        })
        .collect()
}
