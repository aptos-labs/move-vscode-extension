use crate::resolve::check_resolve_tmpfs;
use test_utils::fixtures::test_state::TestPackageFiles;

#[test]
fn test_module_item_cross_tmpfs() {
    check_resolve_tmpfs(vec![TestPackageFiles::named(
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
    check_resolve_tmpfs(vec![TestPackageFiles::new(
        "main",
        // language=TOML
        r#"
[package]
name = "Main"
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
        TestPackageFiles::new(
            "main",
            // language=TOML
            r#"
[package]
name = "Main"

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
        TestPackageFiles::new(
            "m",
            // language=TOML
            r#"
[package]
name = "M"
        "#,
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
        TestPackageFiles::new(
            "main",
            // language=TOML
            r#"
[package]
name = "Main"

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
        TestPackageFiles::new(
            "aptos_std",
            // language=TOML
            r#"
[package]
name = "AptosStd"

[dependencies]
Std = { local = "../std" }
        "#,
            // language=Move
            r#""#,
        ),
        TestPackageFiles::new(
            "std",
            // language=TOML
            r#"
[package]
name = "Std"
        "#,
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
        TestPackageFiles::new(
            "main",
            // language=TOML
            r#"
[package]
name = "Main"

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
        TestPackageFiles::new(
            "aptos_std",
            // language=TOML
            r#"
[package]
name = "AptosStd"

[dependencies]
Std = { local = "../std" }
        "#,
            // language=Move
            r#""#,
        ),
        TestPackageFiles::new(
            "std",
            // language=TOML
            r#"
[package]
name = "Std"
        "#,
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
        TestPackageFiles::new(
            "main",
            // language=TOML
            r#"
[package]
name = "Main"

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
        TestPackageFiles::new(
            "aptos_std",
            // language=TOML
            r#"
[package]
name = "AptosStd"

[dependencies]
Std = { local = "../std" }
        "#,
            // language=Move
            r#""#,
        ),
        TestPackageFiles::new(
            "std",
            // language=TOML
            r#"
[package]
name = "Std"
        "#,
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
        TestPackageFiles::new(
            "main",
            // language=TOML
            r#"
[package]
name = "Main"

[dependencies]
Std = { local = "../std" }
AptosStd = { local = "../aptos_std" }
        "#,
            // language=Move
            r#"
"#,
        ),
        TestPackageFiles::new(
            "aptos_std",
            // language=TOML
            r#"
[package]
name = "AptosStd"

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
        TestPackageFiles::new(
            "std",
            // language=TOML
            r#"
[package]
name = "Std"
        "#,
            // language=Move
            r#"
//- /table.move
module std::table {
    struct Table { val: u8 }
            //X
}
"#,
        ),
        TestPackageFiles::new(
            "std2",
            // language=TOML
            r#"
[package]
name = "Std2"
        "#,
            // language=Move
            r#"
"#,
        ),
    ])
}
