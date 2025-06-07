use std::iter;
use stdx::itertools::Itertools;
use syntax::files::FilePosition;
use test_utils::{fixtures, get_all_marked_positions, get_marked_position_offset_with_data};

#[test]
fn test_find_function_usages() {
    // language=Move
    let source = r#"
module 0x1::m {
    fun call() {
       //^
    }
    fun m1() {
        call();
       //X
    }
    fun m2() {
        call();
       //X
    }
}
    "#;
    let (analysis, file_id) = fixtures::from_single_file(source.to_string());
    let (ref_offset, _) = get_marked_position_offset_with_data(&source, "//^");

    let refs = analysis
        .find_all_refs(FilePosition { file_id, offset: ref_offset }, None)
        .unwrap();
    let actual_ref_ranges = refs
        .expect("no declaration at //^")
        .references
        .get(&file_id)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|range| range)
        .sorted_by_key(|it| it.start())
        .collect::<Vec<_>>();

    let target_offsets = get_all_marked_positions(source, "//X")
        .iter()
        .map(|it| it.item_offset)
        .sorted()
        .collect::<Vec<_>>();

    assert_eq!(
        actual_ref_ranges.len(),
        target_offsets.len(),
        "not all references are found"
    );

    for (actual_ref_range, expected_offset) in iter::zip(actual_ref_ranges, target_offsets) {
        assert!(actual_ref_range.contains(expected_offset))
    }
}
