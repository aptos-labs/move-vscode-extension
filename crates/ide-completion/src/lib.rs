#![allow(dead_code)]

use crate::completions::{item_list, path, Completions};
use crate::config::CompletionConfig;
use crate::context::{CompletionAnalysis, CompletionContext};
use crate::item::CompletionItem;
use ide_db::RootDatabase;
use lang::files::FilePosition;
use std::cell::RefCell;

pub mod completions;
pub mod config;
pub mod context;
pub mod item;
pub mod render;

pub fn completions(
    db: &RootDatabase,
    config: &CompletionConfig,
    position: FilePosition,
) -> Option<Vec<CompletionItem>> {
    let (ctx, analysis) = CompletionContext::new(db, position, config)?;

    let completions = RefCell::new(Completions::default());
    {
        match analysis {
            CompletionAnalysis::Item(item_list_kind) => {
                item_list::complete_item_list(&completions, &ctx, &item_list_kind);
            }
            CompletionAnalysis::Path(path) => path::add_path_completions(&completions, &ctx, path),
        }
    }

    let completions = completions.into_inner();
    Some(completions.into())
}
