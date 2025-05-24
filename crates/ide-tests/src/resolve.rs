use crate::init_tracing_for_test;
use ide::{Analysis, NavigationTarget};
use syntax::AstNode;
use syntax::SyntaxKind::{IDENT, QUOTE_IDENT};
use syntax::files::FilePosition;
use test_utils::fixtures::test_state::TestPackageFiles;
use test_utils::{fixtures, get_first_marked_position, get_marked_position_offset_with_data};
use vfs::FileId;

mod test_resolve_1;
mod test_resolve_fs;
mod test_resolve_functions;
mod test_resolve_loop_labels;
mod test_resolve_modules;
mod test_resolve_receiver_style_function;
mod test_resolve_specs;
mod test_resolve_struct_fields;
mod test_resolve_types;
mod test_resolve_variables;

pub fn check_resolve(source: &str) {
    init_tracing_for_test();

    let (ref_offset, data) = get_marked_position_offset_with_data(&source, "//^");

    let (analysis, file_id) = fixtures::from_single_file(source.to_string());
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
    let nav_item = item.expect("item is unresolved");

    assert_resolves_to_target(&analysis, nav_item, (file_id, source.to_string()));
}

pub fn check_resolve_files(files: &str) {
    init_tracing_for_test();

    let test_package = fixtures::from_multiple_files(files);
    let (ref_file_id, ref_file_text) = test_package.file_with_caret("//^");
    let (ref_offset, data) = get_marked_position_offset_with_data(&ref_file_text, "//^");

    let analysis = test_package.analysis();
    let position = FilePosition {
        file_id: ref_file_id,
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
    let nav_item = item.expect("item is unresolved");

    assert_resolves_to_target(&analysis, nav_item, test_package.file_with_caret("//X"));
}

pub fn check_resolve_tmpfs(test_packages: Vec<TestPackageFiles>) {
    init_tracing_for_test();

    let test_state = fixtures::from_multiple_files_on_tmpfs(test_packages);

    let (ref_file_id, ref_file_text) = test_state.file_with_caret("//^");
    let (ref_offset, data) = get_marked_position_offset_with_data(&ref_file_text, "//^");

    let analysis = test_state.analysis();
    let position = FilePosition {
        file_id: ref_file_id,
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
    let nav_item = item.expect("item is unresolved");

    assert_resolves_to_target(&analysis, nav_item, test_state.file_with_caret("//X"));
}

#[track_caller]
fn assert_resolves_to_target(
    analysis: &Analysis,
    nav_item: NavigationTarget,
    target_file: (FileId, String),
) {
    let (target_file_id, target_file_text) = target_file;

    let target_offset = get_first_marked_position(&target_file_text, "//X").item_offset;
    let target_file = analysis.parse(target_file_id).unwrap();

    let marked_ident_token = target_file
        .syntax()
        .token_at_offset(target_offset)
        .find(|token| matches!(token.kind(), IDENT | QUOTE_IDENT))
        .expect("no ident token on mark");
    let ident_text_range = marked_ident_token.text_range();
    assert_eq!(nav_item.focus_range.unwrap(), ident_text_range);
}
