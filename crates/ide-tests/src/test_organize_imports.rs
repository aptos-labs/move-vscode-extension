// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ide_test_utils::diagnostics::apply_fix;
use expect_test::{Expect, expect};
use test_utils::fixtures;
use test_utils::tracing::init_tracing_for_test;

fn check_organize_imports(before: &str, after: Expect) {
    init_tracing_for_test();

    let before_source = stdx::trim_indent(before);
    let (analysis, file_id) = fixtures::from_single_file(before_source.to_string());

    let organize_assist = analysis
        .organize_imports(file_id)
        .unwrap()
        .expect("no assist found");
    let mut actual_after = apply_fix(&organize_assist, &before_source).trim_end().to_string();
    actual_after.push_str("\n");

    after.assert_eq(&stdx::trim_indent(&actual_after).as_str());
}

#[test]
fn test_remove_unused_type_import() {
    // language=Move
    check_organize_imports(
        r#"
    module 0x1::m {
        struct MyStruct {}
        public fun call() {}
    }
    module 0x1::main {
        use 0x1::m::MyStruct;
        use 0x1::m::call;

        fun main() {
            let a = call();
        }
    }
    "#,
        expect![[r#"
            module 0x1::m {
                struct MyStruct {}
                public fun call() {}
            }
            module 0x1::main {
                use 0x1::m::call;

                fun main() {
                    let a = call();
                }
            }
        "#]],
    )
}

#[test]
fn test_remove_unused_import_from_group_in_the_middle() {
    // language=Move
    check_organize_imports(
        r#"
        module 0x1::M {
            struct MyStruct {}
            public fun call() {}
            public fun aaa() {}
        }
        script {
            use 0x1::M::{aaa, MyStruct, call};

            fun main() {
                let a = call();
                let a = aaa();
            }
        }
    "#,
        expect![[r#"
            module 0x1::M {
                struct MyStruct {}
                public fun call() {}
                public fun aaa() {}
            }
            script {
                use 0x1::M::{aaa, call};

                fun main() {
                    let a = call();
                    let a = aaa();
                }
            }
        "#]],
    )
}

#[test]
fn test_remove_unused_import_from_group_in_the_beginning() {
    // language=Move
    check_organize_imports(
        r#"
        module 0x1::M {
            struct Bbb {}
            public fun call() {}
            public fun aaa() {}
        }
        script {
            use 0x1::M::{aaa, Bbb, call};

            fun main() {
                let a: Bbb = call();
            }
        }
    "#,
        expect![[r#"
            module 0x1::M {
                struct Bbb {}
                public fun call() {}
                public fun aaa() {}
            }
            script {
                use 0x1::M::{Bbb, call};

                fun main() {
                    let a: Bbb = call();
                }
            }
        "#]],
    )
}

#[test]
fn test_remove_unused_import_from_group_in_the_end() {
    // language=Move
    check_organize_imports(
        r#"
        module 0x1::M {
            struct Bbb {}
            public fun call() {}
            public fun aaa() {}
        }
        script {
            use 0x1::M::{aaa, Bbb, call};

            fun main() {
                let a: Bbb = aaa();
            }
        }
    "#,
        expect![[r#"
            module 0x1::M {
                struct Bbb {}
                public fun call() {}
                public fun aaa() {}
            }
            script {
                use 0x1::M::{aaa, Bbb};

                fun main() {
                    let a: Bbb = aaa();
                }
            }
        "#]],
    )
}

#[test]
fn test_remove_redundant_group_curly_braces() {
    // language=Move
    check_organize_imports(
        r#"
        module 0x1::M {
            struct MyStruct {}
            public fun call() {}
        }
        script {
            use 0x1::M::{call};

            fun main() {
                let a = call();
            }
        }
    "#,
        expect![[r#"
            module 0x1::M {
                struct MyStruct {}
                public fun call() {}
            }
            script {
                use 0x1::M::call;

                fun main() {
                    let a = call();
                }
            }
        "#]],
    )
}

#[test]
fn test_remove_redundant_group_curly_braces_with_self() {
    // language=Move
    check_organize_imports(
        r#"
            module 0x1::M {
                struct MyStruct {}
                public fun call() {}
            }
            script {
                use 0x1::M::{Self};

                fun main() {
                    let a = M::call();
                }
            }
    "#,
        expect![[r#"
            module 0x1::M {
                struct MyStruct {}
                public fun call() {}
            }
            script {
                use 0x1::M;

                fun main() {
                    let a = M::call();
                }
            }
        "#]],
    )
}

#[test]
fn test_remove_unused_module_import() {
    // language=Move
    check_organize_imports(
        r#"
            module 0x1::M {}
            module 0x1::M2 {
                use 0x1::M;
            }
    "#,
        expect![[r#"
            module 0x1::M {}
            module 0x1::M2 {
                }
        "#]],
    )
}

#[test]
fn test_remove_unused_import_group_with_two_imports() {
    // language=Move
    check_organize_imports(
        r#"
            module 0x1::M {
                struct BTC {}
                struct USDT {}
            }
            module 0x1::Main {
                use 0x1::M::{BTC, USDT};
            }
    "#,
        expect![[r#"
            module 0x1::M {
                struct BTC {}
                struct USDT {}
            }
            module 0x1::Main {
                }
        "#]],
    )
}

#[test]
fn test_remove_all_imports_if_not_needed() {
    // language=Move
    check_organize_imports(
        r#"
            module Std::Errors {}
            module Std::Signer {}
            module AAA::M1 {
                struct S1 {}
                struct SS1 {}
            }
            module BBB::M2 {
                struct S2 {}
            }
            module 0x1::Main {
                use Std::Errors;
                use Std::Signer;

                use AAA::M1::S1;
                use AAA::M1::SS1;
                use BBB::M2::S2;

                #[test]
                fun call() {}
            }
    "#,
        expect![[r#"
            module Std::Errors {}
            module Std::Signer {}
            module AAA::M1 {
                struct S1 {}
                struct SS1 {}
            }
            module BBB::M2 {
                struct S2 {}
            }
            module 0x1::Main {
                #[test]
                fun call() {}
            }
        "#]],
    )
}

#[test]
fn test_remove_empty_group() {
    // language=Move
    check_organize_imports(
        r#"
            module 0x1::M1 {}
            module 0x1::Main {
                use 0x1::M1::{};
            }
    "#,
        expect![[r#"
            module 0x1::M1 {}
            module 0x1::Main {
                }
        "#]],
    )
}

#[test]
fn test_module_spec() {
    // language=Move
    check_organize_imports(
        r#"
            module 0x1::string {}
            spec 0x1::main {
                use 0x1::string;
            }
    "#,
        expect![[r#"
            module 0x1::string {}
            spec 0x1::main {
                }
        "#]],
    )
}

#[test]
fn test_duplicate_struct_import() {
    // language=Move
    check_organize_imports(
        r#"
module 0x1::pool {
    struct X1 {}
    public fun create_pool<BinStep>() {}
}
module 0x1::main {
    use 0x1::pool::{Self, X1, X1};

    fun main() {
        pool::create_pool<X1>();
    }
}
    "#,
        expect![[r#"
            module 0x1::pool {
                struct X1 {}
                public fun create_pool<BinStep>() {}
            }
            module 0x1::main {
                use 0x1::pool::{Self, X1};

                fun main() {
                    pool::create_pool<X1>();
                }
            }
    "#]],
    )
}

#[test]
fn test_unused_import_with_self_as_in_group() {
    // language=Move
    check_organize_imports(
        r#"
module 0x1::pool {
    struct X1 {}
    public fun create_pool() {}
}
module 0x1::main {
    use 0x1::pool::{Self as mypool, X1};

    fun main() {
        mypool::create_pool();
    }
}
    "#,
        expect![[r#"
            module 0x1::pool {
                struct X1 {}
                public fun create_pool() {}
            }
            module 0x1::main {
                use 0x1::pool as mypool;

                fun main() {
                    mypool::create_pool();
                }
            }
    "#]],
    )
}

#[test]
fn test_unused_import_with_self_as_in_group_with_extra_items() {
    // language=Move
    check_organize_imports(
        r#"
module 0x1::pool {
    struct X1 {}
    struct X2 {}
    public fun create_pool() {}
}
module 0x1::main {
    use 0x1::pool::{Self as mypool, X1, X2};

    fun main(x: X2) {
        mypool::create_pool();
    }
}
    "#,
        expect![[r#"
            module 0x1::pool {
                struct X1 {}
                struct X2 {}
                public fun create_pool() {}
            }
            module 0x1::main {
                use 0x1::pool::{Self as mypool, X2};

                fun main(x: X2) {
                    mypool::create_pool();
                }
            }
    "#]],
    )
}

#[test]
fn test_simplify_self() {
    // language=Move
    check_organize_imports(
        r#"
module 0x1::pool {
    struct X1 {}
    public fun create_pool() {}
}
module 0x1::main {
    use 0x1::pool::Self;

    fun main() {
        pool::create_pool();
    }
}
    "#,
        expect![[r#"
            module 0x1::pool {
                struct X1 {}
                public fun create_pool() {}
            }
            module 0x1::main {
                use 0x1::pool;

                fun main() {
                    pool::create_pool();
                }
            }
    "#]],
    )
}

#[test]
fn test_simplify_self_as() {
    // language=Move
    check_organize_imports(
        r#"
module 0x1::pool {
    struct X1 {}
    public fun create_pool() {}
}
module 0x1::main {
    use 0x1::pool::Self as mypool;

    fun main() {
        mypool::create_pool();
    }
}
    "#,
        expect![[r#"
            module 0x1::pool {
                struct X1 {}
                public fun create_pool() {}
            }
            module 0x1::main {
                use 0x1::pool as mypool;

                fun main() {
                    mypool::create_pool();
                }
            }
    "#]],
    )
}

#[test]
fn test_duplicate_self_import() {
    // language=Move
    check_organize_imports(
        r#"
        module 0x1::pool {
            struct X1 {}
            public fun create_pool<BinStep>() {}
        }
        module 0x1::main {
            use 0x1::pool::{Self, Self, X1};

            fun main() {
                pool::create_pool<X1>();
            }
        }
    "#,
        expect![[r#"
            module 0x1::pool {
                struct X1 {}
                public fun create_pool<BinStep>() {}
            }
            module 0x1::main {
                use 0x1::pool::{Self, X1};

                fun main() {
                    pool::create_pool<X1>();
                }
            }
        "#]],
    )
}
