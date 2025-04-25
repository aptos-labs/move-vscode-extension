use crate::test_utils::check_diagnostic;
use crate::test_utils::diagnostics::check_no_diagnostics;

#[test]
fn test_unresolved_variable() {
    // language=Move
    check_diagnostic(
        r#"
module 0x1::main {
    fun main() {
        x;
      //^ err: Unresolved reference `x`
    }
}
"#,
    );
}

#[test]
fn test_unresolved_function_call() {
    // language=Move
    check_diagnostic(
        r#"
module 0x1::main {
    fun main() {
        call();
      //^^^^ err: Unresolved reference `call`
    }
}
"#,
    );
}

#[test]
fn test_unresolved_module_member_with_unresolved_module() {
    // language=Move
    check_diagnostic(
        r#"
module 0x1::main {
    use 0x1::mod::call;
           //^^^ err: Unresolved reference `mod`

    fun main() {
        call();
      //^^^^ err: Unresolved reference `call`
    }
}
"#,
    );
}

#[test]
fn test_no_unresolved_reference_for_builtin() {
    // language=Move
    check_no_diagnostics(
        r#"
module 0x1::m {
    fun main() {
        move_from<u8>(@0x1);
    }
}
"#,
    );
}

#[test]
fn test_no_unresolved_reference_for_primitive_type() {
    // language=Move
    check_no_diagnostics(
        r#"
script {
    fun main(s: &signer) {
    }
}
"#,
    );
}

#[test]
fn test_unresolved_reference_for_variable_in_struct_lit_field() {
    // language=Move
    check_diagnostic(
        r#"
module 0x1::M {
    struct T {
        my_field: u8
    }

    fun main() {
        let t = T { my_field: my_unknown };
                            //^^^^^^^^^^ err: Unresolved reference `my_unknown`
    }
}
"#,
    );
}

#[test]
fn test_no_unresolved_reference_for_field_shorthand() {
    // language=Move
    check_no_diagnostics(
        r#"
module 0x1::M {
    struct T {
        my_field: u8
    }

    fun main() {
        let my_field = 1;
        let t = T { my_field };
    }
}
"#,
    );
}

#[test]
fn test_unresolved_field_in_struct_lit() {
    // language=Move
    check_diagnostic(
        r#"
module 0x1::M {
    struct T {
        my_field: u8
    }

    fun main() {
        let t = T { my_unknown_field: 1 };
                  //^^^^^^^^^^^^^^^^ err: Unresolved reference `my_unknown_field`

    }
}
"#,
    );
}

#[test]
fn test_unresolved_field_in_struct_pat() {
    // language=Move
    check_diagnostic(
        r#"
module 0x1::M {
    struct T {
        my_field: u8
    }

    fun main() {
        let T { my_unknown_field: _ } = T { my_field: 1 };
              //^^^^^^^^^^^^^^^^ err: Unresolved reference `my_unknown_field`

    }
}
"#,
    );
}

#[test]
fn test_unresolved_field_in_struct_pat_shorthand() {
    // language=Move
    check_diagnostic(
        r#"
module 0x1::M {
    struct T {
        my_field: u8
    }

    fun main() {
        let T { my_unknown_field } = T { my_field: 1 };
              //^^^^^^^^^^^^^^^^ err: Unresolved reference `my_unknown_field`

    }
}
"#,
    );
}

#[test]
fn test_unresolved_module() {
    // language=Move
    check_diagnostic(
        r#"
module 0x1::M {
    fun main() {
        let t = transaction::create();
              //^^^^^^^^^^^ err: Unresolved reference `transaction`
    }
}
"#,
    );
}

#[test]
fn test_unresolved_fq_module() {
    // language=Move
    check_diagnostic(
        r#"
module 0x1::M {
    fun main() {
        let t = std::transaction::create();
                   //^^^^^^^^^^^ err: Unresolved reference `transaction`
    }
}
"#,
    );
}
