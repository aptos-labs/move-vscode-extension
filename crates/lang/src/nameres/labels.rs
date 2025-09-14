// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::loc::SyntaxLocFileExt;
use crate::nameres::namespaces::Ns;
use crate::nameres::scope::ScopeEntry;
use syntax::SyntaxKind::*;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};
use syntax::{SyntaxElement, SyntaxKind};

pub fn loop_ancestors(element: &SyntaxElement) -> Vec<ast::LoopLike> {
    let mut loops = vec![];
    for scope in element.ancestors() {
        if is_label_barrier(scope.kind()) {
            break;
        }
        if let Some(loop_ans) = scope.cast::<ast::LoopLike>() {
            loops.push(loop_ans);
        }
    }
    loops
}

#[tracing::instrument(level = "debug", skip(label))]
pub fn get_loop_labels_resolve_variants(label: InFile<ast::Label>) -> Vec<ScopeEntry> {
    let (file_id, label) = label.unpack();

    let mut label_entries = vec![];
    for loop_like in loop_ancestors(&label.syntax().clone().into()) {
        if let Some(label_decl) = loop_like.label_decl() {
            let entry = label_decl_to_entry(label_decl.in_file(file_id));
            label_entries.push(entry);
        }
    }
    tracing::debug!(?label_entries);
    label_entries
}

fn label_decl_to_entry(label_decl: InFile<ast::LabelDecl>) -> ScopeEntry {
    let item_loc = label_decl.loc();
    // anything works here
    let item_ns = Ns::NAME;
    let entry = ScopeEntry {
        name: label_decl.value.name_as_string(),
        node_loc: item_loc,
        ns: item_ns,
        scope_adjustment: None,
    };
    entry
}

fn is_label_barrier(kind: SyntaxKind) -> bool {
    matches!(kind, LAMBDA_EXPR | FUN | SPEC_FUN | SPEC_INLINE_FUN | CONST)
}
