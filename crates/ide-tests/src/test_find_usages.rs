use std::iter;
use stdx::itertools::Itertools;
use syntax::files::FilePosition;
use test_utils::{fixtures, get_all_marked_positions, get_marked_position_offset_with_data};

fn check_find_usages(source: &str) {
    let (analysis, file_id) = fixtures::from_single_file(source.to_string());
    let (ref_offset, _) = get_marked_position_offset_with_data(&source, "//^");

    let ref_position = FilePosition { file_id, offset: ref_offset };
    let target = analysis
        .goto_definition(ref_position)
        .unwrap()
        .expect("item should resolve to itself");
    assert!(target.range.contains(ref_offset), "item should resolve to itself");

    let refs = analysis.find_all_refs(ref_position, None).unwrap();
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

#[test]
fn test_function_usages() {
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
    check_find_usages(source)
}

#[test]
fn test_struct_usages() {
    // language=Move
    let source = r#"
module 0x1::m {
    struct MyStruct { val: u8 }
           //^
    fun m1(a: MyStruct) {
             //X
        let MyStruct { val } =
             //X
            MyStruct { val: 1 };
           //X
    }
}
    "#;
    check_find_usages(source)
}

#[test]
fn test_function_parameter_usages() {
    // language=Move
    let source = r#"
module 0x1::m {
    fun main(a: u8) {
           //^
        a;
      //X
        call(a);
           //X
    }
}
spec 0x1::m {
    spec main(a: u8) {
            //X
        a;
      //X
    }
}
    "#;
    check_find_usages(source)
}

#[test]
fn test_lambda_parameter_usages() {
    // language=Move
    let source = r#"
module 0x1::m {
    fun main() {
        |a| {
       //^
            a;
          //X
            call(a);
               //X
        }
    }
}
    "#;
    check_find_usages(source)
}

#[test]
fn test_let_variable_usages() {
    // language=Move
    let source = r#"
module 0x1::m {
    fun main() {
        a;
        let a = 1;
          //^
        a;
      //X
        call(a);
           //X
    }
}
    "#;
    check_find_usages(source)
}
