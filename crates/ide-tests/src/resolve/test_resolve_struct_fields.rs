use crate::resolve::check_resolve;

#[test]
fn test_resolve_field_from_struct_lit() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
            struct T {
                my_field: u8
              //X
            }

            fun main() {
                let t = T { my_field: 1 };
                          //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_field_from_struct_pat() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
            struct T {
                my_field: u8
              //X
            }

            fun main() {
                let T { my_field: my_field_1 } = call();
                      //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_field_from_struct_pat_shorthand() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
            struct T {
                my_field: u8
              //X
            }

            fun main() {
                let T { my_field } = call();
                      //^
            }
        }
    "#,
    )
}
