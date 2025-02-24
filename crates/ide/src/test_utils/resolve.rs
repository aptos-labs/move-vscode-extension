use crate::test_utils::{get_marked_position_offset, get_marked_position_offset_with_data};
use crate::Analysis;
use lang::files::FilePosition;
use syntax::SyntaxKind::IDENT;
use syntax::{AstNode, SyntaxKind};

pub fn check_resolve(source: &str) {
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
    // file.syntax().token_at_offset()
    // let target_item_name =
    //     algo::find_node_at_offset::<ast::Name>(&file.syntax(), target_offset).unwrap();

    assert_eq!(item.focus_range.unwrap(), ident_text_range);
}
