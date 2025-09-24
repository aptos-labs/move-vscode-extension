// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

#![allow(dead_code)]

use crate::completions::{Completions, item_list, reference};
use crate::config::CompletionConfig;
use crate::context::{CompletionAnalysis, CompletionContext};
use crate::item::CompletionItem;
use ide_db::source_change::SourceChangeBuilder;
use ide_db::text_edit::TextEdit;
use ide_db::{RootDatabase, imports};
use lang::Semantics;
use lang::item_scope::ItemScope;
use std::cell::RefCell;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::files::FilePosition;
use syntax::{AstNode, ast};

pub mod completions;
pub mod config;
pub mod context;
pub mod item;
pub mod render;

pub fn completions(
    db: &RootDatabase,
    config: &CompletionConfig,
    position: FilePosition,
    _trigger_character: Option<char>,
) -> Option<Vec<CompletionItem>> {
    let (ctx, analysis) = CompletionContext::new_with_analysis(db, position, config)?;

    let completions = RefCell::new(Completions::default());
    {
        match analysis {
            CompletionAnalysis::Item(item_list_kind) => {
                item_list::complete_item_list(&completions, &ctx, &item_list_kind);
            }
            CompletionAnalysis::Reference(reference_kind) => {
                reference::add_reference_completions(&completions, &ctx, reference_kind);
            }
            CompletionAnalysis::TypeParam => {
                let generic_item = ctx
                    .original_file()?
                    .find_node_at_offset::<ast::TypeParamList>(position.offset)?
                    .generic_element()?;
                // let generic_element = type_param_list.generic_element()?;
                if matches!(
                    generic_item,
                    ast::GenericElement::Struct(_) | ast::GenericElement::Enum(_)
                ) {
                    completions
                        .borrow_mut()
                        .add_keyword_snippet(&ctx, "phantom", "phantom $0");
                }
            }
        }
    }

    let completions = completions.into_inner();
    Some(completions.into())
}

/// Resolves additional completion data at the position given.
/// This is used for import insertion done via completions like flyimport and custom user snippets.
pub fn resolve_completion_edits(
    db: &RootDatabase,
    FilePosition { file_id, offset }: FilePosition,
    import_to_add: String,
    item_scope: ItemScope,
) -> Option<Vec<TextEdit>> {
    let _p = tracing::info_span!("resolve_completion_edits").entered();
    let sema = Semantics::new(db, file_id);

    let original_file = sema.parse(file_id);
    let original_token = syntax::AstNode::syntax(&original_file)
        .token_at_offset(offset)
        .left_biased()?;
    let position_for_import = &original_token.parent()?;

    let items_owner = position_for_import.containing_items_owner()?;

    let mut builder = SourceChangeBuilder::new(file_id);

    let mut editor = builder.make_editor(items_owner.syntax());
    let add_imports = imports::add_import_for_import_path(&items_owner, import_to_add, Some(item_scope));
    add_imports(&mut editor);
    builder.add_file_edits(file_id, editor);

    let source_change = builder.finish();
    let text_edit = source_change.get_source_edit(file_id)?.clone();
    Some(vec![text_edit])
}
