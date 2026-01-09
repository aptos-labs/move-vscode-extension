use expect_test::expect;
use stdx::itertools::Itertools;
use test_utils::fixtures;
use test_utils::fixtures::test_state::package;

#[test]
fn test_collect_named_addresses_from_packages() {
    let test_state = fixtures::from_multiple_files_on_tmpfs(vec![
        package(
            "MyApp",
            // language=TOML
            r#"
            [package]
            name = "MyApp"
            version = "0.1.0"

            [dependencies]
            MyDep = { local = "../MyDep" }
            AptosTokenObjects = { local = "../AptosTokenObjects" }

            [addresses]
            my_app_address = "_"
            "#,
            r#"
            //- main.move
            /*caret*/
            "#,
        ),
        package(
            "MyDep",
            // language=TOML
            r#"
            [package]
            name = "MyDep"
            version = "0.1.0"

            [dependencies]
            MySourceDep = { local = "../MySourceDep" }

            [addresses]
            my_dep_address = "_"
            "#,
            r#""#,
        ),
        package(
            "MySourceDep",
            // language=TOML
            r#"
            [package]
            name = "MySourceDep"
            version = "0.1.0"

            [addresses]
            my_source_dep_address = "_"

            [dependencies]
            "#,
            r#""#,
        ),
        package(
            "AptosTokenObjects",
            // language=TOML
            r#"
            [package]
            name = "AptosTokenObjects"
            version = "0.1.0"

            [addresses]
            std = "0x1"
            aptos_std = "0x1"
            aptos_framework = "0x1"
            aptos_token_objects = "0x4"
            "#,
            r#""#,
        ),
    ]);
    let named_addresses = test_state
        .analysis()
        .named_addresses()
        .unwrap()
        .into_iter()
        .sorted()
        .collect::<Vec<_>>();
    let expected = expect![[r#"
        [
            (
                "aptos_framework",
                "0x1",
            ),
            (
                "aptos_std",
                "0x1",
            ),
            (
                "aptos_token",
                "0x1",
            ),
            (
                "aptos_token_objects",
                "0x4",
            ),
            (
                "my_app_address",
                "_",
            ),
            (
                "my_dep_address",
                "_",
            ),
            (
                "my_source_dep_address",
                "_",
            ),
            (
                "std",
                "0x1",
            ),
        ]"#]];
    expected.assert_eq(&format!("{:#?}", named_addresses))
}
