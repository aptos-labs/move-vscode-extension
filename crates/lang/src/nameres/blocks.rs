// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::loc::{SyntaxLocFileExt, SyntaxLocInput};
use crate::nameres::scope::{NamedItemsExt, NamedItemsInFileExt, ScopeEntry};
use base_db::SourceDatabase;
use std::cell::LazyCell;
use stdx::itertools::Itertools;
use syntax::ast::HasStmts;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::InFile;
use syntax::{AstNode, SyntaxNode, TextRange, TextSize, ast};

pub fn get_entries_in_block(
    db: &dyn SourceDatabase,
    block_expr: InFile<ast::BlockExpr>,
    start_at: &SyntaxNode,
) -> Vec<ScopeEntry> {
    let mut entries = vec![];
    let start_at_offset = start_at.text_range().start();

    let is_msl = block_expr.is_msl();
    if is_msl {
        let spec_inline_funs = block_expr.map_ref(|it| it.spec_inline_functions()).flatten();
        entries.extend(spec_inline_funs.to_entries());
    }

    let let_stmts = let_stmts_with_bindings(db, block_expr);
    // make it lazy to not call it in non-msl case (most common)
    let current_let_stmt = LazyCell::new(|| start_at.ancestor_of_type::<ast::LetStmt>(false));
    let bindings = let_stmts
        .into_iter()
        .filter(|(let_stmt_info, _)| {
            if !is_msl {
                return let_stmt_info.strictly_before(start_at_offset);
            }
            if let Some(current_let_stmt) = current_let_stmt.as_ref() {
                let is_post_visible = !let_stmt_info.is_post || current_let_stmt.is_post();
                return is_post_visible && let_stmt_info.strictly_before(start_at_offset);
            }
            true
        })
        .collect::<Vec<_>>();

    let binding_entries_with_shadowing = bindings
        .into_iter()
        .rev()
        .flat_map(|(_, bindings)| bindings)
        // shadowing
        .unique_by(|e| e.name.clone());
    entries.extend(binding_entries_with_shadowing);

    entries
}

#[derive(Clone, Eq, PartialEq)]
struct LetStmtInfo {
    is_post: bool,
    text_range: TextRange,
}

impl LetStmtInfo {
    fn strictly_before(&self, offset: TextSize) -> bool {
        self.text_range.end() <= offset
    }
}

fn let_stmts_with_bindings(
    db: &dyn SourceDatabase,
    block: InFile<ast::BlockExpr>,
) -> Vec<(LetStmtInfo, Vec<ScopeEntry>)> {
    let block_loc = SyntaxLocInput::new(db, block.loc());
    let_stmts_with_bindings_tracked(db, block_loc).unwrap_or_default()
}

#[salsa_macros::tracked]
fn let_stmts_with_bindings_tracked(
    db: &dyn SourceDatabase,
    block_loc: SyntaxLocInput<'_>,
) -> Option<Vec<(LetStmtInfo, Vec<ScopeEntry>)>> {
    let (file_id, block) = block_loc.to_ast::<ast::BlockExpr>(db)?.unpack();
    let let_stmts_infos = block
        .let_stmts()
        .map(|let_stmt| {
            let bindings = let_stmt.pat().map(|pat| pat.bindings()).unwrap_or_default();
            let let_stmt_info = LetStmtInfo {
                is_post: let_stmt.is_post(),
                text_range: let_stmt.syntax().text_range(),
            };
            (let_stmt_info, bindings.to_entries(file_id))
        })
        .collect::<Vec<_>>();
    Some(let_stmts_infos)
}
