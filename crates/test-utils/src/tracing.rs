use tracing::Level;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};
use tracing_tree::HierarchicalLayer;

pub fn init_tracing_for_test() {
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
