// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use expect_test::{Expect, expect};
use ide_db::text_edit::TextEdit;
use syntax::files::FilePosition;
use test_utils::{fixtures, get_marked_position_offset_with_data};

fn check_rename(before: &str, rename_to: &str, after: Expect) {
    let before = before.to_string();
    // let before = stdx::trim_indent(before);

    let (offset, _) = get_marked_position_offset_with_data(&before, "//^");
    let (analysis, file_id) = fixtures::from_single_file(before);
    let position = FilePosition { file_id, offset };

    if !after.data().starts_with("// error: ") {
        if let Err(err) = analysis.prepare_rename(position).unwrap() {
            panic!("Prepare rename to '{rename_to}' was failed: {err}")
        }
    }

    let rename_result = analysis
        .rename(position, &rename_to)
        .unwrap_or_else(|err| panic!("Rename to '{rename_to}' was cancelled: {err}"));
    match rename_result {
        Ok(source_change) => {
            let mut text_edit_builder = TextEdit::builder();
            let (&file_id, edit) = match source_change.source_file_edits.len() {
                0 => return,
                1 => source_change.source_file_edits.iter().next().unwrap(),
                _ => panic!(),
            };
            for text_edit in edit.iter() {
                text_edit_builder.replace(text_edit.range, text_edit.new_text.clone());
            }
            let mut result = analysis.file_text(file_id).unwrap().to_string();
            text_edit_builder.finish().apply(&mut result);

            let mut actual_after = result.trim().to_string();
            actual_after.push_str("\n");

            after.assert_eq(&actual_after);
        }
        Err(err) => {
            if after.data().trim_start().starts_with("// error:") {
                let error_message = format!("// error: {err}\n");
                after.assert_eq(&error_message.trim_start());
            } else {
                panic!("Rename to '{rename_to}' failed unexpectedly: {err}")
            }
        }
    }
}

#[test]
fn test_rename_function() {
    check_rename(
        // language=Move
        r#"
module 0x1::m {
    fun call() {
       //^
    }
    fun main() {
        call();
    }
}
    "#,
        "my_call",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun my_call() {
                   //^
                }
                fun main() {
                    my_call();
                }
            }
        "#]],
    );
}

#[test]
fn test_cannot_rename_to_0x1() {
    check_rename(
        // language=Move
        r#"
            module 0x1::m {
                fun call() {
                   //^
                }
                fun main() {
                    call();
                }
            }
    "#,
        "0x1",
        // language=Move
        expect![[r#"
            // error: Invalid name `0x1`: not an identifier
        "#]],
    );
}

#[test]
fn test_cannot_rename_to_keyword() {
    check_rename(
        // language=Move
        r#"
            module 0x1::m {
                fun call() {
                   //^
                }
                fun main() {
                    call();
                }
            }
    "#,
        "let",
        // language=Move
        expect![[r#"
            // error: Invalid name `let`: cannot rename to a keyword
        "#]],
    );
}

#[test]
fn test_rename_struct_field() {
    check_rename(
        // language=Move
        r#"
module 0x1::m {
    struct S { val: u8 }
              //^
    fun main() {
        S { val: 1 }.val;
    }
}
    "#,
        "my_val",
        // language=Move
        expect![[r#"
            module 0x1::m {
                struct S { my_val: u8 }
                          //^
                fun main() {
                    S { my_val: 1 }.my_val;
                }
            }
        "#]],
    );
}

#[test]
fn test_rename_struct_field_with_shorthand() {
    check_rename(
        // language=Move
        r#"
module 0x1::m {
    struct S { val: u8 }
              //^
    fun main() {
        let val = 1;
        S { val };
    }
}
    "#,
        "my_val",
        // language=Move
        expect![[r#"
            module 0x1::m {
                struct S { my_val: u8 }
                          //^
                fun main() {
                    let val = 1;
                    S { my_val: val };
                }
            }
        "#]],
    );
}

#[test]
fn test_rename_ident_pat_with_shorthand() {
    check_rename(
        // language=Move
        r#"
module 0x1::m {
    struct S { val: u8 }
    fun main() {
        let val = 1;
           //^
        S { val };
    }
}
    "#,
        "my_val",
        // language=Move
        expect![[r#"
            module 0x1::m {
                struct S { val: u8 }
                fun main() {
                    let my_val = 1;
                       //^
                    S { val: my_val };
                }
            }
        "#]],
    );
}

#[test]
fn test_rename_struct_field_back_to_shorthand() {
    check_rename(
        // language=Move
        r#"
module 0x1::m {
    struct S { my_val: u8 }
              //^
    fun main() {
        let val = 1;
        S { my_val: val };
    }
}
        "#,
        "val",
        // language=Move
        expect![[r#"
            module 0x1::m {
                struct S { val: u8 }
                          //^
                fun main() {
                    let val = 1;
                    S { val };
                }
            }
        "#]],
    );
}
