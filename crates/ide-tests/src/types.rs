use crate::init_tracing_for_test;
use ide::Analysis;
use ide::test_utils::get_marked_position_offset_with_data;
use lang::FilePosition;

mod test_types_expression_types;

pub fn check_expr_type(source: &str) {
    init_tracing_for_test();

    let (analysis, file_id) = Analysis::from_single_file(source.to_string());

    let (ref_offset, data) = get_marked_position_offset_with_data(&source, "//^");
    let position = FilePosition {
        file_id,
        offset: ref_offset,
    };

    let opt_ty = analysis.expr_type_info(position).unwrap();
    let expr_ty = opt_ty.expect("should be an expr at the file position");

    assert_eq!(expr_ty, data);
    // assert_eq!(expr_ty, data, "expected `{}`, actual `{}`", expr_ty, data);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr_type() {
        // language=Move
        check_expr_type(
            r#"
module 0x1::m {
    fun call<T>(val: T): T {
        val
    }
    fun main() {
        call(1u8);
       //^ u8
    }
}
"#,
        );
    }
}
