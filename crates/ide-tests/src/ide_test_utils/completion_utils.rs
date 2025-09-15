// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::init_tracing_for_test;
use expect_test::Expect;
use ide::Analysis;
use ide_completion::config::CompletionConfig;
use ide_completion::item::CompletionItem;
use ide_db::AllowSnippets;
use syntax::files::FilePosition;
use syntax::{AstNode, T, TextSize};
use test_utils::{fixtures, get_and_replace_caret};
use vfs::FileId;

pub fn do_single_completion(before: &str, after: Expect) {
    do_single_completion_with_config(
        CompletionConfig {
            allow_snippets: AllowSnippets::new(true),
            ..CompletionConfig::default()
        },
        before,
        after,
    )
}

pub fn do_single_completion_with_config(
    completion_config: CompletionConfig,
    before: &str,
    after: Expect,
) {
    let trimmed_before = stdx::trim_indent(before).trim().to_string();
    let (source, offset) = get_and_replace_caret(&trimmed_before, "/*caret*/");

    let (analysis, file_id, mut completion_items) =
        completions_at_offset(&source, offset, &completion_config, true);
    match completion_items.len() {
        0 => {
            panic!("no completions returned")
        }
        1 => (),
        _ => {
            panic!(
                "multiple completions returned {:?}",
                lookup_labels(completion_items)
            );
        }
    }

    let completion_item = completion_items.pop().unwrap();

    let mut res = source.to_string();
    completion_item.text_edit.apply(&mut res);

    if let Some(import_to_add) = completion_item.import_to_add {
        let text_edits = analysis
            .resolve_completion_edits(
                &completion_config,
                FilePosition { file_id, offset },
                import_to_add,
            )
            .unwrap();
        for text_edit in text_edits {
            text_edit.apply(&mut res);
        }
    }

    let mut res = res.replace("$0", "/*caret*/");
    res.push_str("\n");

    after.assert_eq(&res);
}

pub fn check_completions(source: &str, expected: Expect) {
    init_tracing_for_test();

    let (source, offset) = get_and_replace_caret(source, "/*caret*/");

    let (_, _, mut completion_items) = completions_at_offset(
        source,
        offset,
        &CompletionConfig {
            allow_snippets: AllowSnippets::new(true),
            ..CompletionConfig::default()
        },
        true,
    );
    completion_items.sort_by_key(|it| it.relevance.score() ^ 0xFF_FF_FF_FF);

    let lookup_labels_txt = format!("{:#?}", lookup_labels(completion_items));
    expected.assert_eq(&lookup_labels_txt);
}

pub fn check_completions_with_config(
    completion_config: CompletionConfig,
    source: &str,
    expected: Expect,
) {
    init_tracing_for_test();

    let (source, offset) = get_and_replace_caret(source, "/*caret*/");

    let (_, _, mut completion_items) = completions_at_offset(source, offset, &completion_config, true);
    completion_items.sort_by_key(|it| it.relevance.score() ^ 0xFF_FF_FF_FF);

    let lookup_labels_txt = format!("{:#?}", lookup_labels(completion_items));
    expected.assert_eq(&lookup_labels_txt);
}

pub fn check_no_completions(source: &str) {
    let (source, caret_offset) = get_and_replace_caret(source, "/*caret*/");
    let (_, _, completion_items) = completions_at_offset(
        source,
        caret_offset,
        &CompletionConfig {
            allow_snippets: AllowSnippets::new(true),
            ..CompletionConfig::default()
        },
        true,
    );
    assert!(
        completion_items.is_empty(),
        "extra completion items {:?}",
        lookup_labels(completion_items),
    );
}

fn completions_at_offset(
    source: &str,
    caret_offset: TextSize,
    completion_config: &CompletionConfig,
    filter_with_prefix: bool,
) -> (Analysis, FileId, Vec<CompletionItem>) {
    let (analysis, file_id) = fixtures::from_single_file(source.to_string());

    let source_file = analysis.parse(file_id).unwrap();

    let file_position = FilePosition {
        file_id,
        offset: caret_offset,
    };
    let mut completion_items = analysis
        .completions(completion_config, file_position, None)
        .unwrap()
        .unwrap_or_default();

    if filter_with_prefix {
        let token_at_offset = source_file.syntax().token_at_offset(caret_offset).left_biased();
        if let Some(token_at_offset) = token_at_offset {
            let is_word = token_at_offset.kind().is_keyword() || token_at_offset.kind() == T![ident];
            let prefix = if is_word {
                let rel_offset = caret_offset - token_at_offset.text_range().start();
                token_at_offset.text()[0..rel_offset.into()].to_string()
            } else {
                "".to_string()
            };
            completion_items.retain(|item| item.lookup().split(" ").any(|it| it.starts_with(&prefix)))
        }
    }

    (analysis, file_id, completion_items)
}

fn lookup_labels(items: Vec<CompletionItem>) -> Vec<String> {
    items
        .iter()
        .map(|item| {
            let mut label = item.label.primary.clone();
            if let Some(detail) = &item.detail {
                label += &format!(" -> {}", detail);
            }
            label
        })
        .collect()
}
