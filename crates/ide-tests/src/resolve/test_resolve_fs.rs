use crate::resolve::check_resolve_tmpfs;
use test_utils::fixtures::global_state::TestPackageFiles;

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
