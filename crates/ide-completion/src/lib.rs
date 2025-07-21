// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

#![allow(dead_code)]

use crate::completions::{Completions, item_list, reference};
use crate::config::CompletionConfig;
use crate::context::{CompletionAnalysis, CompletionContext};
use crate::item::CompletionItem;
use ide_db::RootDatabase;
use std::cell::RefCell;
use syntax::ast;
use syntax::files::FilePosition;

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
                let type_param_list = ctx
                    .original_file()?
                    .find_node_at_offset::<ast::TypeParamList>(position.offset)?;
                let generic_element = type_param_list.generic_element()?;
                if matches!(
                    generic_element,
                    ast::GenericElement::Struct(_) | ast::GenericElement::Enum(_)
                ) {
                    let acc = &mut completions.borrow_mut();
                    acc.add_keyword_snippet(&ctx, "phantom", "phantom $0");
                }
            }
        }
    }

    let completions = completions.into_inner();
    Some(completions.into())
}
