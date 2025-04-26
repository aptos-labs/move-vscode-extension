use crate::init_tracing_for_test;
use ide::Analysis;
use ide::test_utils::{get_first_marked_position, get_marked_position_offset_with_data};
use syntax::SyntaxKind::IDENT;
use syntax::files::FilePosition;
use syntax::{AstNode, SyntaxKind};

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

    let ident_token = file
        .syntax()
        .token_at_offset(target_offset)
        .find(|token| token.kind() == IDENT)
        .unwrap();
    let ident_parent = ident_token.parent().unwrap();
    let ident_text_range = match ident_parent.kind() {
        SyntaxKind::NAME => ident_parent.text_range(),
        _ => panic!(
            "//X does not point to named item, actual {:?}",
            ident_parent.kind()
        ),
    };
    assert_eq!(item.focus_range.unwrap(), ident_text_range);
}
