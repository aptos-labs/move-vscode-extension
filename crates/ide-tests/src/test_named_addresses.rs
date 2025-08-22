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
    ]);
    let (file_id, _) = test_state.file_with_caret("/*caret*/");

    let named_addresses = test_state
        .analysis()
        .named_addresses(file_id)
        .unwrap()
        .into_iter()
        .sorted()
        .collect::<Vec<_>>();
    let expected = expect![[r#"
        [
            "aptos_experimental",
            "aptos_framework",
            "aptos_std",
            "aptos_token",
            "my_app_address",
            "my_dep_address",
            "my_source_dep_address",
            "std",
        ]"#]];
    expected.assert_eq(&format!("{:#?}", named_addresses))
}
