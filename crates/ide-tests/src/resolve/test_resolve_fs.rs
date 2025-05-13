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
