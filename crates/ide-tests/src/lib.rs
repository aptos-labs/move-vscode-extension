#![cfg(test)]

use tracing::Level;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};
use tracing_tree::HierarchicalLayer;

mod fold;
mod hover;
mod resolve;
mod test_completion;
mod test_inlay_hints;
mod test_replace_with_compound_expr;
mod test_replace_with_method_call;
mod test_syntax_highlighting;
mod test_unresolved_reference;
mod test_utils;
mod types;

mod test_completion_functions;
mod test_completion_methods;
mod test_load_dependencies;
mod test_resolve_items;
mod test_resolve_types;
mod test_syntax_errors;
mod test_type_checking;
mod test_view_syntax_tree;

pub(crate) fn init_tracing_for_test() {
    let _ = Registry::default()
        // .with(fmt::Layer::new().with_max_level(Level::DEBUG))
        .with(
            HierarchicalLayer::new(2)
                .with_indent_lines(true)
                .with_deferred_spans(true)
                .with_filter(LevelFilter::from_level(Level::DEBUG)),
        )
        .try_init();
}
