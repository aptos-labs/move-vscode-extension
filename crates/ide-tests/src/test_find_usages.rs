use lang::nameres::scope::VecExt;
use stdx::itertools::Itertools;
use syntax::files::FilePosition;
use test_utils::{
    SourceMark, apply_source_marks, fixtures, get_all_marked_positions,
    get_marked_position_offset_with_data,
};

fn check_find_usages(source: &str) {
    let (analysis, file_id) = fixtures::from_single_file(source.to_string());
    let (ref_offset, _) = get_marked_position_offset_with_data(&source, "//^");

    let ref_position = FilePosition { file_id, offset: ref_offset };
    let targets = analysis
        .goto_definition_multi(ref_position)
        .unwrap()
        .expect("item is unresolved");
    let target = targets
        .info
        .single_or_none()
        .expect("item should resolve only to itself");
    assert!(
        target.focus_range.is_some_and(|it| it.contains(ref_offset)),
        "item should resolve to itself"
    );

    let refs = analysis.find_all_refs(ref_position, None).unwrap();
    let mut actual_ref_ranges = refs
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

    let mut missing_target_offsets = vec![];
    for target_offset in target_offsets {
        let ref_range_pos = actual_ref_ranges.iter().position(|it| it.contains(target_offset));
        match ref_range_pos {
            Some(pos) => {
                actual_ref_ranges.remove(pos);
            }
            None => {
                missing_target_offsets.push(target_offset);
            }
        }
    }
    assert!(
        missing_target_offsets.is_empty(),
        "not all references are found: \n{}",
        {
            let missing_marks = missing_target_offsets
                .into_iter()
                .map(|offset| SourceMark::at_offset(offset, "missing reference"))
                .collect();
            apply_source_marks(&source, missing_marks)
        }
    );
    assert!(actual_ref_ranges.is_empty(), "extra references found: \n{}", {
        let extra_marks = actual_ref_ranges
            .into_iter()
            .map(|range| SourceMark::at_range(range, "extra reference"))
            .collect();
        apply_source_marks(&source, extra_marks)
    });
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
    struct Struct { a: u8 }
    fun main() {
        a;
        let a = 1;
          //^
        a;
      //X
        call(a);
           //X
        Struct { a }
               //X
    }
}
    "#;
    check_find_usages(source)
}

#[test]
fn test_struct_field_usages() {
    // language=Move
    let source = r#"
module 0x1::m {
    struct S { val: u8 }
              //^
    fun main(s: S) {
        s.val;
         //X
        let S { val } = s;
               //X
        let S { val: myval } = s;
               //X
        S { val: 1 };
            //X
        S { val };
            //X
    }
}
    "#;
    check_find_usages(source)
}

#[test]
fn test_schema_find_usages() {
    // language=Move
    let source = r#"
module 0x1::m {
    spec schema  S {
        val: u8;
    }
    spec schema T {
        my_val: u8;
        //^
        include S { val: my_val };
                         //X
    }
}
    "#;
    check_find_usages(source)
}

#[test]
fn test_schema_find_usages_shorthand() {
    // language=Move
    let source = r#"
module 0x1::m {
    spec schema  S {
        val: u8;
    }
    spec schema T {
        val: u8;
       //^
        include S { val };
                    //X
    }
}
    "#;
    check_find_usages(source)
}
