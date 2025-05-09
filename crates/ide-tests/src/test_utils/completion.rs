use crate::init_tracing_for_test;
use expect_test::Expect;
use ide::Analysis;
use ide_completion::config::CompletionConfig;
use ide_completion::item::CompletionItem;
use ide_db::SnippetCap;
use syntax::files::FilePosition;
use syntax::{AstNode, AstToken, TextSize, ast};
use test_utils::get_and_replace_caret;

pub fn do_single_completion(before: &str, after: Expect) {
    let trimmed_before = stdx::trim_indent(before).trim().to_string();
    let (source, offset) = get_and_replace_caret(&trimmed_before, "/*caret*/");

    let completion_items = completions_at_offset(&trimmed_before, offset, true);
    assert_eq!(
        completion_items.len(),
        1,
        "multiple completions returned {:?}",
        lookup_labels(completion_items)
    );
    let completion_item = completion_items.first().unwrap();

    let mut res = source.to_string();
    completion_item.text_edit.apply(&mut res);
    res.push_str("\n");

    after.assert_eq(&res);
}

pub fn check_completions_with_prefix_exact(source: &str, expected_items: Vec<&str>) {
    init_tracing_for_test();

    let (source, caret_offset) = get_and_replace_caret(source, "/*caret*/");
    let completion_items = completions_at_offset(source, caret_offset, true);

    let mut lookup_labels = lookup_labels(completion_items);
    for item in expected_items {
        let item = item.to_string();
        assert!(lookup_labels.contains(&item), "missing item '{}'", item);
        lookup_labels.retain(|lookup| *lookup != item);
    }

    assert!(lookup_labels.is_empty(), "extra items {:?}", lookup_labels);
}

pub fn check_completions_contains(source: &str, contains_items: Vec<&str>) {
    init_tracing_for_test();

    let (source, offset) = get_and_replace_caret(source, "/*caret*/");

    let completion_items = completions_at_offset(source, offset, false);

    let mut lookup_labels = lookup_labels(completion_items);
    let lookup_labels_txt = format!("{:?}", lookup_labels);
    for item in contains_items.clone() {
        let item = item.to_string();
        assert!(
            lookup_labels.contains(&item),
            "missing item '{}', actual: {}",
            item,
            lookup_labels_txt
        );
        lookup_labels.retain(|lookup| *lookup != item);
    }

    // assert!(lookup_labels.is_empty(), "extra items {:?}", lookup_labels);
}

pub fn check_completion_exact(source: &str, expected_items: Vec<&str>) {
    let completion_items = completions_at_caret(source);

    let mut lookup_labels = lookup_labels(completion_items);
    for expected_item in expected_items {
        let item = expected_item.to_string();
        assert!(lookup_labels.contains(&item), "missing item '{}'", expected_item);
        lookup_labels.retain(|lookup| *lookup != item);
    }

    assert!(lookup_labels.is_empty(), "extra items {:?}", lookup_labels);
}

pub fn check_no_completions(source: &str) {
    let completion_items = completions_at_caret(source);
    assert!(
        completion_items.is_empty(),
        "extra completion items {:?}",
        lookup_labels(completion_items),
    );
}

fn completions_at_caret(source: &str) -> Vec<CompletionItem> {
    let (source, caret_offset) = get_and_replace_caret(source, "/*caret*/");
    completions_at_offset(source, caret_offset, false)
}

fn completions_at_offset(
    source: &str,
    caret_offset: TextSize,
    filter_with_prefix: bool,
) -> Vec<CompletionItem> {
    let (analysis, file_id) = Analysis::from_single_file(source.to_string());

    let source_file = analysis.parse(file_id).unwrap();

    let file_position = FilePosition {
        file_id,
        offset: caret_offset,
    };
    let completion_config = CompletionConfig {
        snippet_cap: SnippetCap::new(true),
        ..CompletionConfig::default()
    };
    let mut completion_items = analysis
        .completions(&completion_config, file_position, None)
        .unwrap()
        .unwrap_or_default();

    if filter_with_prefix {
        if let Some(t) = source_file.syntax().token_at_offset(caret_offset).left_biased() {
            if let Some(ident_token) = ast::Ident::cast(t) {
                let prefix = ident_token.text().to_string();
                completion_items.retain(|item| item.label.primary.starts_with(&prefix))
            };
        }
    }

    completion_items
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
