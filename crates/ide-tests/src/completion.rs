mod test_completion;

use crate::assert_eq_text;
use crate::test_utils::get_and_replace_caret;
use ide::Analysis;
use ide_completion::config::CompletionConfig;
use ide_completion::item::CompletionItem;
use ide_db::SnippetCap;
use lang::files::FilePosition;
use syntax::{ast, AstNode, AstToken, TextSize};
use tracing::level_filters::LevelFilter;
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};
use tracing_tree::HierarchicalLayer;

pub fn do_single_completion(before: &str, after: &str) {
    let (source, offset) = get_and_replace_caret(before, "/*caret*/");

    let completion_items = completions_at_offset(before, offset, true);
    assert_eq!(
        completion_items.len(),
        1,
        "multiple completions returned {:?}",
        lookup_labels(completion_items)
    );
    let completion_item = completion_items.first().unwrap();

    let mut res = source.to_string();
    completion_item.text_edit.apply(&mut res);

    assert_eq_text!(after, &res);
}

pub fn check_completions_with_prefix_exact(source: &str, expected_items: Vec<&str>) {
    let _ = Registry::default()
        // .with(fmt::Layer::new().with_max_level(Level::DEBUG))
        .with(HierarchicalLayer::new(2).with_filter(LevelFilter::from_level(Level::DEBUG)))
        .try_init();

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
    let (source, offset) = get_and_replace_caret(source, "/*caret*/");

    let completion_items = completions_at_offset(source, offset, false);

    let mut lookup_labels = lookup_labels(completion_items);
    for item in contains_items {
        let item = item.to_string();
        assert!(lookup_labels.contains(&item), "missing item '{}'", item);
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
        .completions(&completion_config, file_position)
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
    items.iter().map(|item| item.label.primary.clone()).collect()
}
