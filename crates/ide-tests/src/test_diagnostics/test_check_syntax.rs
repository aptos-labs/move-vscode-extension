use crate::ide_test_utils::diagnostics::check_diagnostics;
use expect_test::expect;

#[test]
fn test_spec_fun_with_explicit_return_value() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            spec fun value(): u8 { 1 }
            spec fun vec(): vector<u8> { vector[1] }
            spec native fun vec(): vector<u8>;
        }
    "#]]);
}

#[test]
fn test_spec_fun_requires_return_value() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            spec fun value() {}
                          //^^ err: Spec function requires return type
            spec native fun vec();
                               //^ err: Spec function requires return type
        }
    "#]]);
}

#[test]
fn test_entry_fun_without_return_value() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            public entry fun transfer() {}
            public entry fun transfer_params(a: u8, b: u8) { a + b; }
        }
    "#]]);
}

#[test]
fn test_error_if_entry_fun_has_return_value() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            public entry fun transfer(): u8 { 1 }
                                     //^^^^ err: Entry functions cannot return values
            public entry fun transfer_params(a: u8, b: u8): (u8, u8) { (a, b) }
                                                        //^^^^^^^^^^ err: Entry functions cannot return values
        }
    "#]]);
}

#[test]
fn test_no_error_if_test_function() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            #[test_only]
            public entry fun main1(): u8 { 1 }
            #[test]
            public entry fun main2(): u8 { 1 }
        }
    "#]]);
}
