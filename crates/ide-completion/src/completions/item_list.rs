// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::completions::Completions;
use crate::context::CompletionContext;
use std::cell::RefCell;
use std::collections::HashSet;
use syntax::ast;

/// The kind of item list a [`PathKind::Item`] belongs to.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ItemListKind {
    SourceFile,
    Module,
    Function { existing_modifiers: HashSet<String> },
    Ability { existing_abilities: HashSet<String> },
}

pub(crate) fn complete_item_list(
    acc: &RefCell<Completions>,
    ctx: &CompletionContext,
    kind: &ItemListKind,
) {
    let _p = tracing::info_span!("complete_item_list", ?kind).entered();
    add_keywords(acc, ctx, kind);
}

fn add_keywords(acc: &RefCell<Completions>, ctx: &CompletionContext, kind: &ItemListKind) -> Option<()> {
    let add_keyword = |kw: &str| {
        let snippet = if ctx.next_char_is(' ') {
            format!("{kw}$0")
        } else {
            format!("{kw} $0")
        };
        acc.borrow_mut().add_keyword_snippet(ctx, kw, snippet.leak());
    };
    let add_keyword_no_space = |kw: &str| {
        let snippet = format!("{}$0", kw);
        acc.borrow_mut().add_keyword_snippet(ctx, kw, snippet.leak())
    };

    match kind {
        ItemListKind::SourceFile => {
            add_keyword("module");
            add_keyword("script");
            add_keyword("spec");
        }
        ItemListKind::Module => {
            add_keyword("use");
            add_keyword("fun");
            add_keyword("struct");
            add_keyword("const");
            add_keyword("enum");
            add_keyword("spec");
            add_keyword("friend");

            for function_modifier in all_function_modifiers().into_iter() {
                if function_modifier == "friend" {
                    continue;
                }
                add_keyword(&function_modifier);
            }

            if let Some(struct_) = ctx.prev_ast_node::<ast::Struct>()
                && struct_.field_list().is_none()
                && struct_.ability_list().is_none()
            {
                add_keyword("has");
            }
        }
        ItemListKind::Function { existing_modifiers } => {
            for function_modifier in all_function_modifiers() {
                if existing_modifiers.contains(&function_modifier) {
                    continue;
                }
                add_keyword(&function_modifier);
            }
            add_keyword("fun");
        }
        ItemListKind::Ability { existing_abilities } => {
            let all_abilities = vec!["key", "store", "copy", "drop"]
                .into_iter()
                .map(|it| it.to_string())
                .filter(|it| !existing_abilities.contains(it));
            for ability in all_abilities {
                add_keyword_no_space(&ability);
            }
        }
    }

    Some(())
}

fn all_function_modifiers() -> Vec<String> {
    vec!["public", "native", "entry", "inline", "package", "friend"]
        .into_iter()
        .map(|it| it.to_string())
        .collect()
}
