use crate::init_tracing_for_test;
use ide::Analysis;
use ide::test_utils::{get_first_marked_position, get_marked_position_offset_with_data};
use syntax::AstNode;
use syntax::SyntaxKind::{IDENT, QUOTE_IDENT};
use syntax::files::FilePosition;

mod test_resolve_1;
mod test_resolve_functions;
mod test_resolve_loop_labels;
mod test_resolve_modules;
mod test_resolve_receiver_style_function;
mod test_resolve_specs;
mod test_resolve_struct_fields;
mod test_resolve_types;
mod test_resolve_variables;

#[track_caller]
pub(crate) fn check_resolve(source: &str) {
    init_tracing_for_test();

    let (ref_offset, data) = get_marked_position_offset_with_data(&source, "//^");

    let (analysis, file_id) = Analysis::from_single_file(source.to_string());
    let position = FilePosition { file_id, offset: ref_offset };

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

    let target_offset = get_first_marked_position(&source, "//X").item_offset;
    let file = analysis.parse(file_id).unwrap();

    let marked_ident_token = file
        .syntax()
        .token_at_offset(target_offset)
        .find(|token| matches!(token.kind(), IDENT | QUOTE_IDENT))
        .expect("no ident token on mark");
    let ident_text_range = marked_ident_token.text_range();
    assert_eq!(item.focus_range.unwrap(), ident_text_range);
}
