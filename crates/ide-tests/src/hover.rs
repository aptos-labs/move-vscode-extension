use crate::init_tracing_for_test;
use ide::test_utils::get_marked_position_offset_with_data;
use ide::Analysis;
use lang::FilePosition;

pub(crate) fn check_hover(source: &str, expected_docs: &str) {
    init_tracing_for_test();

    let (ref_offset, data) = get_marked_position_offset_with_data(&source, "//^");

    let (analysis, file_id) = Analysis::from_single_file(source.to_string());
    let position = FilePosition {
        file_id,
        offset: ref_offset,
    };

    let hover_result = analysis
        .hover(position)
        .unwrap()
        .map(|range_info| range_info.info);
    let hover_result = hover_result.expect("no docs");

    let doc_string: String = hover_result.doc_string.into();
    assert_eq!(doc_string, expected_docs);
}

#[test]
fn test_hover_for_function() {
    check_hover(
        // language=Move
        r#"
module 0x1::m {
    /// my documentation string
    fun main() {
        //^
    }
}
    "#, "my documentation string")
}