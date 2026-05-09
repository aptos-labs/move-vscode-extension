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
fn test_apply_top_level_lemma_from_proof() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            spec lemma add_zero_right() {
                        //X
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

#[test]
fn test_proof_resolve_let_variable() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            fun main() {}
            spec main {} proof {
                let acc = 1;
                   //X
                acc;
               //^
            }
        }
    "#,
    )
}

#[test]
fn test_proof_resolve_let_variable_with_post() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            fun main() {}
            spec main {} proof {
                post let acc1 = 1;
                        //X
                post acc1;
                    //^
            }
        }
    "#,
    )
}

#[test]
fn test_proof_resolve_post_has_access_to_pre_lets_inline() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            fun main() {}
            spec main {} proof {
                let acc1 = 1;
                   //X
                post acc1;
                    //^
            }
        }
    "#,
    )
}

#[test]
fn test_proof_resolve_post_has_access_to_pre_lets_block() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            fun main() {}
            spec main {} proof {
                let acc1 = 1;
                   //X
                post { acc1; }
                      //^
            }
        }
    "#,
    )
}

#[test]
fn test_proof_resolve_let_variable_in_post_block() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            fun main() {}
            spec main {} proof {
                post {
                    let acc = 1;
                        //X
                    acc;
                   //^
                }
            }
        }
    "#,
    )
}

#[test]
fn test_pre_expr_cant_access_post_let() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            fun main() {}
            spec main {} proof {
                post {
                    let acc = 1;
                }
                acc1;
              //^ unresolved
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_forall_apply_variables() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::main {
            fun main() {}
            spec lemma add_mono(a: u64) {}
            spec main {} proof {
                forall a: u64
                     //X
                    apply add_mono(a);
                                 //^
            }
        }
    "#,
    )
}
