use crate::resolve::check_resolve_tmpfs;
use test_utils::fixtures::test_state::{named, named_with_deps};

#[test]
fn test_module_item_cross_tmpfs() {
    check_resolve_tmpfs(vec![named(
        "WsRoot",
        // language=Move
        r#"
//- /m.move
module std::m {
    public fun call() {}
              //X
}
//- /main.move
module std::main {
    use std::m::call;
    public fun main() {
        call();
       //^
    }
}
"#,
    )])
}

#[test]
fn test_cross_package_resolve_with_public_package() {
    check_resolve_tmpfs(vec![named(
        "main",
        // language=Move
        r#"
//- /main.move
module std::main {
    use std::m::call;
    public fun main() {
        call();
       //^
    }
}
//- /m.move
module std::m {
    public(package) fun call() {}
                       //X
}
"#,
    )])
}

#[test]
fn test_cross_package_resolve() {
    check_resolve_tmpfs(vec![
        named_with_deps(
            "main",
            // language=TOML
            r#"
[dependencies]
M = { local = "../m"}
        "#,
            // language=Move
            r#"
//- /main.move
module std::main {
    use std::m::call;
    public fun main() {
        call();
       //^
    }
}
"#,
        ),
        named(
            "m",
            // language=Move
            r#"
//- /m.move
module std::m {
    public fun call() {}
              //X
}
"#,
        ),
    ])
}

#[test]
fn test_transitive_dependency() {
    check_resolve_tmpfs(vec![
        named_with_deps(
            "main",
            // language=TOML
            r#"
[dependencies]
AptosStd = { local = "../aptos_std" }
        "#,
            // language=Move
            r#"
//- /main.move
module std::main {
    use std::table::Table;
    public fun main(t: Table) {
                      //^
    }
}
"#,
        ),
        named_with_deps(
            "aptos_std",
            // language=TOML
            r#"
[dependencies]
Std = { local = "../std" }
        "#,
            // language=Move
            r#""#,
        ),
        named(
            "std",
            // language=Move
            r#"
//- /table.move
module std::table {
    struct Table { val: u8 }
            //X
}
"#,
        ),
    ])
}

#[test]
fn test_resolve_to_item_which_occurs_in_dep_tree_twice() {
    check_resolve_tmpfs(vec![
        named_with_deps(
            "main",
            // language=TOML
            r#"
[dependencies]
AptosStd = { local = "../aptos_std" }
Std = { local = "../std" }
        "#,
            // language=Move
            r#"
//- /main.move
module std::main {
    use std::table::Table;
    public fun main(t: Table) {
                      //^
    }
}
"#,
        ),
        named_with_deps(
            "aptos_std",
            // language=TOML
            r#"
[dependencies]
Std = { local = "../std" }
        "#,
            // language=Move
            r#""#,
        ),
        named(
            "std",
            // language=Move
            r#"
//- /table.move
module std::table {
    struct Table { val: u8 }
            //X
}
"#,
        ),
    ])
}

#[test]
fn test_cannot_resolve_dependency_if_no_toml_declaration() {
    check_resolve_tmpfs(vec![
        named_with_deps(
            "main",
            // language=TOML
            r#"
[dependencies]
        "#,
            // language=Move
            r#"
//- /main.move
module std::main {
    use std::table::Table;
    public fun main(t: Table) {
                      //^ unresolved
    }
}
"#,
        ),
        named_with_deps(
            "aptos_std",
            // language=TOML
            r#"
[dependencies]
Std = { local = "../std" }
        "#,
            // language=Move
            r#""#,
        ),
        named(
            "std",
            // language=Move
            r#"
//- /table.move
module std::table {
    struct Table { val: u8 }
}
"#,
        ),
    ])
}

#[test]
fn test_resolve_to_item_in_dep_after_circular() {
    check_resolve_tmpfs(vec![
        named_with_deps(
            "main",
            r#"
[dependencies]
Std = { local = "../std" }
AptosStd = { local = "../aptos_std" }
        "#,
            r#""#,
        ),
        named_with_deps(
            "aptos_std",
            r#"
[dependencies]
Std = { local = "../std" }
Std2 = { local = "../std2" }
        "#,
            // language=Move
            r#"
//- /main.move
module std::main {
    use std::table::Table;
    public fun main(t: Table) {
                      //^
    }
}
"#,
        ),
        named(
            "std",
            // language=Move
            r#"
//- /table.move
module std::table {
    struct Table { val: u8 }
            //X
}
"#,
        ),
        named("std2", r#""#),
    ])
}

#[test]
fn test_resolve_spec_fun_from_related_module_spec() {
    let test_packages = vec![named(
        "AptosStd",
        // language=Move
        r#"
//- /any.move
module std::any {
    fun main() {
        spec {
            use std::from_bcs;
            from_bcs::deserializable();
                     //^
        }
    }
}
//- /from_bcs.move
module std::from_bcs {}

//- /from_bcs.spec.move
spec std::from_bcs {
    spec module {
        fun deserializable(bytes: vector<u8>): bool;
            //X
    }
}
        "#,
    )];
    check_resolve_tmpfs(test_packages);
}
