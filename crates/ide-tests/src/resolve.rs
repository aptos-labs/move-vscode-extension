use ide::test_utils::{get_marked_position_offset, get_marked_position_offset_with_data};
use ide::Analysis;
use lang::FilePosition;
use syntax::SyntaxKind::IDENT;
use syntax::{AstNode, SyntaxKind};
use tracing::metadata::LevelFilter;
use tracing::{Instrument, Level};
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};
use tracing_tree::HierarchicalLayer;

mod test_resolve_functions;
mod test_resolve_modules;
mod test_resolve_type_params;
mod test_resolve_variables;

pub fn check_resolve(source: &str) {
    // let subscriber = Registry::default().with(HierarchicalLayer::new(2));
    // tracing::subscriber::set_global_default(subscriber).unwrap();

    Registry::default()
        // .with(fmt::Layer::new().with_max_level(Level::DEBUG))
        .with(HierarchicalLayer::new(2)
            .with_filter(LevelFilter::from_level(Level::DEBUG)))
        .try_init();
    // let subscriber = Registry::default().with(fmt::Layer::default());
    // tracing_subscriber::fmt()
    // subscriber
    //     .with_max_level(tracing::Level::DEBUG)
    //     .without_time()
    //     .init();

    let (ref_offset, data) = get_marked_position_offset_with_data(&source, "//^");

    let (analysis, file_id) = Analysis::from_single_file(source.to_string());
    let position = FilePosition {
        file_id,
        offset: ref_offset,
    };

    let item = analysis
        .goto_definition(position)
        .unwrap()
        .map(|range_info| range_info.info);
    if data == "unresolved" {
        assert!(
            item.is_none(),
            "Should be unresolved, but instead resolved to {:?}",
            item.unwrap()
        );
        return;
    }
    let item = item.expect("item is unresolved");

    let target_offset = get_marked_position_offset(&source, "//X");
    let file = analysis.parse(file_id).unwrap();

    let ident_token = file
        .syntax()
        .token_at_offset(target_offset)
        .find(|token| token.kind() == IDENT)
        .unwrap();
    let ident_parent = ident_token.parent().unwrap();
    let ident_text_range = match ident_parent.kind() {
        SyntaxKind::NAME => ident_parent.text_range(),
        SyntaxKind::NAME_REF => ident_parent.text_range(),
        _ => panic!(
            "//X does not point to named item, actual {:?}",
            ident_parent.kind()
        ),
    };
    assert_eq!(item.focus_range.unwrap(), ident_text_range);
}
