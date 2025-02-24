use ide::test_utils::resolve::check_resolve;

#[test]
fn test_primitive_type_is_unresolved() {
    check_resolve(
        // language=Move
        r#"
module 0x1::m {
    fun main() {
        let a: u8 = 1;
             //^ unresolved
    }
}
    "#,
    )
}

#[test]
fn test_resolve_to_struct_from_let_stmt() {
    check_resolve(
        // language=Move
        r#"
module 0x1::m {
    struct S { val: u8 }
         //X
    fun main() {
        let a: S = 1;
             //^
    }
}
    "#,
    )
}

#[test]
fn test_resolve_to_struct_from_parameter() {
    check_resolve(
        // language=Move
        r#"
module 0x1::m {
    struct S { val: u8 }
         //X
    fun main(a: S) {
              //^
    }
}
    "#,
    )
}

#[test]
fn test_resolve_to_use_item() {
    check_resolve(
        // language=Move
        r#"
module 0x1::m {
    use 0x1::m1::S;
               //X
    fun main() {
        let a: S = 1;
             //^
    }
}
    "#,
    )
}

#[test]
fn test_resolve_to_use_item_in_group() {
    check_resolve(
        // language=Move
        r#"
module 0x1::m {
    use 0x1::m1::{S};
                //X
    fun main() {
        let a: S = 1;
             //^
    }
}
    "#,
    )
}

#[test]
fn test_resolve_struct_type() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    struct S { val: u8 }
         //X
    fun main() {
        let S { val } = 1;
          //^
    }
}
    "#,
    );
}

#[test]
fn test_cannot_resolve_type_to_variable() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    fun main() {
        let S = 1;
        let S { val } = 1;
          //^ unresolved
    }
}
    "#,
    );
}

#[test]
fn test_resolve_to_type_parameter() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    fun main<Element>() {
             //X
        let a: Element;
               //^
    }
}
    "#,
    );
}
