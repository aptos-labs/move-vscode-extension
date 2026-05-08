use crate::resolve::check_resolve;

#[test]
fn test_proof_has_access_to_function_type_parameters() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            fun main<Type>() {}
                   //X
            spec main {} proof {
                let a: Type = 1;
                     //^
            }
        }
    "#,
    )
}

#[test]
fn test_proof_has_access_to_function_parameters() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            fun main(acc: &signer) {}
                   //X
            spec main {} proof {
                acc;
                //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_lemma_variable() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            spec module {
                lemma add_zero_right(x: u64) {
                                   //X
                    ensures x + 0 == x;
                          //^
                }
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_lemma_type_param() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            spec module {
                lemma add_zero_right<T>() {
                                   //X
                    let a: T = 1;
                         //^
                }
            }
        }
    "#,
    )
}

#[test]
fn test_apply_lemma_from_proof() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            spec module {
                lemma add_zero_right() {
                        //X
                }
            }
            fun main() {}
            spec main {} proof {
                apply add_zero_right();
                       //^
            }
        }
    "#,
    )
}

#[test]
fn test_apply_lemma_from_its_own_proof() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            spec module {
                lemma add_zero_right() {
                        //X
                } proof {
                    apply add_zero_right();
                           //^
                }
            }
        }
    "#,
    )
}

#[test]
fn test_spec_fun_accessible_from_lemma() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            spec fun sum(n: num): num {
                    //X
                n
            }
            spec module {
                lemma add_zero_right() {
                    ensures sum(n) == n;
                          //^
                }
            }
        }
    "#,
    )
}

#[test]
fn test_spec_fun_accessible_from_lemma_proof() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            spec fun sum(n: num): num {
                    //X
                n
            }
            spec module {
                lemma add_zero_right() {
                } proof {
                    ensures sum(n) == n;
                          //^
                }
            }
        }
    "#,
    )
}

#[test]
fn test_lemma_accessible_forall_apply() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            spec fun sum(n: num): num {
                n
            }
            fun main() {}
            spec main {} proof {
                forall x: num {sum(x)} apply add_zero_right();
                                                //^
            }
            spec module {
                lemma add_zero_right() {
                         //X
                } proof {
                }
            }
        }
    "#,
    )
}

#[test]
fn test_spec_fun_accessible_forall_triggers() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            spec fun sum(n: num): num {
                    //X
                n
            }
            fun main() {}
            spec main {} proof {
                forall x: num {sum(x)} apply add_zero_right();
                              //^
            }
            spec module {
                lemma add_zero_right() {
                } proof {
                }
            }
        }
    "#,
    )
}
