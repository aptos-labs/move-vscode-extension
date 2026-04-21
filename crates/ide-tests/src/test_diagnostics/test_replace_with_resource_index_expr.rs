// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ide_test_utils::diagnostics::{check_diagnostics, check_diagnostics_and_fix};
use expect_test::expect;

#[test]
fn test_no_trigger_if_res_has_no_key_ability() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct Field has copy { val: u8 }
            struct Res { field: Field }
            fun main(): Field {
                borrow_global<Res>(@0x1).field
                            //^^^ err: Type `0x1::main::Res` does not have required ability `key`
            }
        }
    "#]]);
}

#[test]
fn test_no_trigger_if_parameter_is_not_an_address() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct Field has store, copy { val: u8 }
            struct Res has key { field: Field }
            fun main(): Field {
                borrow_global<Res>(1).field
                                 //^ err: Incompatible type 'integer', expected 'address'
            }
        }
    "#]]);
}

#[test]
fn test_replace_borrow_global_field_with_resource_index_expr() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::main {
                struct Field has copy, store { val: u8 }
                struct Res has key { field: Field }
                fun main(): Field {
                    borrow_global<Res>(@0x1).field
                  //^^^^^^^^^^^^^^^^^^^^^^^^ weak: Replace with resource index expr
                }
            }
        "#]],
        expect![[r#"
        module 0x1::main {
            struct Field has copy, store { val: u8 }
            struct Res has key { field: Field }
            fun main(): Field {
                Res[@0x1].field
            }
        }
    "#]],
    );
}

#[test]
fn test_replace_borrow_global_field_with_resource_index_expr_and_borrow() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::main {
                struct Field has store { val: u8 }
                struct Res has key { field: Field }
                fun main(): &Field {
                    &borrow_global<Res>(@0x1).field
                   //^^^^^^^^^^^^^^^^^^^^^^^^ weak: Replace with resource index expr
                }
            }
        "#]],
        expect![[r#"
        module 0x1::main {
            struct Field has store { val: u8 }
            struct Res has key { field: Field }
            fun main(): &Field {
                &Res[@0x1].field
            }
        }
    "#]],
    );
}

#[test]
fn test_replace_borrow_global_field_with_resource_index_expr_and_borrow_mut() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::main {
                struct Field has store { val: u8 }
                struct Res has key { field: Field }
                fun main(): &mut Field {
                    &mut borrow_global_mut<Res>(@0x1).field
                       //^^^^^^^^^^^^^^^^^^^^^^^^^^^^ weak: Replace with resource index expr
                }
            }
        "#]],
        expect![[r#"
        module 0x1::main {
            struct Field has store { val: u8 }
            struct Res has key { field: Field }
            fun main(): &mut Field {
                &mut Res[@0x1].field
            }
        }
    "#]],
    );
}

#[test]
fn test_replace_borrow_global_field_with_resource_index_expr_and_borrow_mut_mutation_context() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::main {
                struct Res has key { field: u8 }
                fun main() {
                    borrow_global_mut<Res>(@0x1).field = 1;
                  //^^^^^^^^^^^^^^^^^^^^^^^^^^^^ weak: Replace with resource index expr
                }
            }
        "#]],
        expect![[r#"
            module 0x1::main {
                struct Res has key { field: u8 }
                fun main() {
                    Res[@0x1].field = 1;
                }
            }
        "#]],
    );
}

#[test]
fn test_replace_if_field_in_mutation_context() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
        module 0x1::main {
            struct Res has key { field: u8 }
            fun main() {
                borrow_global_mut<Res>(@0x1).field = 1;
              //^^^^^^^^^^^^^^^^^^^^^^^^^^^^ weak: Replace with resource index expr
            }
        }
    "#]],
        expect![[r#"
        module 0x1::main {
            struct Res has key { field: u8 }
            fun main() {
                Res[@0x1].field = 1;
            }
        }
    "#]],
    );
}

#[test]
fn test_replace_borrow_global_with_resource_index_expr_and_borrow_if_top_level_init() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::main {
                struct Res has key { }
                fun main(): &Res {
                    let res = borrow_global<Res>(@0x1);
                            //^^^^^^^^^^^^^^^^^^^^^^^^ weak: Replace with resource index expr
                    res
                }
            }
        "#]],
        expect![[r#"
            module 0x1::main {
                struct Res has key { }
                fun main(): &Res {
                    let res = &Res[@0x1];
                    res
                }
            }
        "#]],
    );
}

#[test]
fn test_replace_borrow_global_with_resource_index_expr_and_borrow_if_tail_expr() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::main {
                struct Res has key { }
                fun main(): &Res {
                    borrow_global<Res>(@0x1)
                  //^^^^^^^^^^^^^^^^^^^^^^^^ weak: Replace with resource index expr
                }
            }
        "#]],
        expect![[r#"
            module 0x1::main {
                struct Res has key { }
                fun main(): &Res {
                    &Res[@0x1]
                }
            }
        "#]],
    );
}

#[test]
fn test_replace_borrow_global_mut_with_resource_index_expr_and_borrow_mut_if_top_level_init() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::main {
                struct Res has key { }
                fun main(): &mut Res {
                    let res = borrow_global_mut<Res>(@0x1);
                            //^^^^^^^^^^^^^^^^^^^^^^^^^^^^ weak: Replace with resource index expr
                    res
                }
            }
        "#]],
        expect![[r#"
            module 0x1::main {
                struct Res has key { }
                fun main(): &mut Res {
                    let res = &mut Res[@0x1];
                    res
                }
            }
        "#]],
    );
}

#[test]
fn test_replace_borrow_global_mut_with_resource_index_expr_and_borrow_mut_if_tail_expr() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::main {
                struct Res has key { }
                fun main(): &mut Res {
                    borrow_global_mut<Res>(@0x1)
                  //^^^^^^^^^^^^^^^^^^^^^^^^^^^^ weak: Replace with resource index expr
                }
            }
        "#]],
        expect![[r#"
            module 0x1::main {
                struct Res has key { }
                fun main(): &mut Res {
                    &mut Res[@0x1]
                }
            }
        "#]],
    );
}

#[test]
fn test_replace_borrow_global_mut_in_mutation_context() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
        module 0x1::main {
            struct Res has key { field: u8 }
            fun main() {
                *borrow_global_mut<Res>(@0x1) = Res { field: 1 };
               //^^^^^^^^^^^^^^^^^^^^^^^^^^^^ weak: Replace with resource index expr
            }
        }
    "#]],
        expect![[r#"
        module 0x1::main {
            struct Res has key { field: u8 }
            fun main() {
                *&mut Res[@0x1] = Res { field: 1 };
            }
        }
    "#]],
    );
}

#[test]
fn test_borrow_global_in_spec() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
        module 0x1::main {
            struct Res has key { field: u8 }
            fun main() {
            }
            spec main {
                let _res = borrow_global<Res>(@0x1);
                         //^^^^^^^^^^^^^^^^^^^^^^^^ weak: Replace with resource index expr
            }
        }
    "#]],
        expect![[r#"
        module 0x1::main {
            struct Res has key { field: u8 }
            fun main() {
            }
            spec main {
                let _res = Res[@0x1];
            }
        }
    "#]],
    );
}

#[test]
fn test_borrow_global_mut_in_spec() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
        module 0x1::main {
            struct Res has key { field: u8 }
            fun main() {
            }
            spec main {
                let _res = borrow_global_mut<Res>(@0x1);
                         //^^^^^^^^^^^^^^^^^^^^^^^^^^^^ weak: Replace with resource index expr
            }
        }
    "#]],
        expect![[r#"
        module 0x1::main {
            struct Res has key { field: u8 }
            fun main() {
            }
            spec main {
                let _res = Res[@0x1];
            }
        }
    "#]],
    );
}

#[test]
fn test_borrow_global_with_read_method_call() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::main {
                struct Res has key { field: u8 }
                fun read_method(self: &Res): u8 { self.field }
                fun main(): u8 {
                    borrow_global<Res>(@0x1).read_method()
                  //^^^^^^^^^^^^^^^^^^^^^^^^ weak: Replace with resource index expr
                }
            }
        "#]],
        expect![[r#"
            module 0x1::main {
                struct Res has key { field: u8 }
                fun read_method(self: &Res): u8 { self.field }
                fun main(): u8 {
                    Res[@0x1].read_method()
                }
            }
        "#]],
    );
}

#[test]
fn test_borrow_global_mut_with_read_method_call() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::main {
                struct Res has key { field: u8 }
                fun read_method(self: &Res): u8 { self.field }
                fun main(): u8 {
                    borrow_global_mut<Res>(@0x1).read_method()
                  //^^^^^^^^^^^^^^^^^^^^^^^^^^^^ weak: Replace with resource index expr
                }
            }
        "#]],
        expect![[r#"
            module 0x1::main {
                struct Res has key { field: u8 }
                fun read_method(self: &Res): u8 { self.field }
                fun main(): u8 {
                    Res[@0x1].read_method()
                }
            }
        "#]],
    );
}

#[test]
fn test_borrow_global_mut_with_write_method_call() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::main {
                struct Res has key { field: u8 }
                fun write_method(self: &mut Res, f: u8) { self.field = f; }
                fun main() {
                    borrow_global_mut<Res>(@0x1).write_method(1)
                  //^^^^^^^^^^^^^^^^^^^^^^^^^^^^ weak: Replace with resource index expr
                }
            }
        "#]],
        expect![[r#"
            module 0x1::main {
                struct Res has key { field: u8 }
                fun write_method(self: &mut Res, f: u8) { self.field = f; }
                fun main() {
                    Res[@0x1].write_method(1)
                }
            }
        "#]],
    );
}
