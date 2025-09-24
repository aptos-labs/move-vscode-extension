// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ide_test_utils::diagnostics::check_diagnostics_on_tmpfs;
use expect_test::expect;
use test_utils::fixtures;
use test_utils::fixtures::test_state::{named, named_with_deps, raw};

#[test]
fn test_types_from_two_different_aptos_framework_dependencies_are_compatible() {
    let test_packages = vec![
        named(
            "AptosStd", // language=Move
            r#""#,
        ),
        raw(
            "AptosFramework",
            "aptos-framework-1",
            // language=Move
            r#"
//- fungible_asset.move
module std::fungible_asset {
    struct FungibleAsset {
        val: u8
    }
}
        "#,
        ),
        raw(
            "AptosFramework",
            "aptos-framework-2",
            // language=Move
            r#"
//- fungible_asset.move
module std::fungible_asset {
    struct FungibleAsset {
        val: u8
    }
}
        "#,
        ),
        named_with_deps(
            "TokenMessenger",
            // language=TOML
            r#"
[dependencies]
AptosStd = { local = "../AptosStd"}
AptosFramework = { local = "../aptos-framework-1"}
        "#,
            // language=Move
            r#"
//- messenger.move
module std::messenger {
    public fun get_fa(_fa: std::fungible_asset::FungibleAsset) {

    }
}
            "#,
        ),
        named_with_deps(
            "Main",
            // language=TOML
            r#"
[dependencies]
AptosFramework = { local = "../aptos-framework-2"}
TokenMessenger = { local = "../TokenMessenger"}
        "#,
            // language=Move
            r#"
//- main.move
module std::main {
    use std::messenger;
    public fun main(fa: std::fungible_asset::FungibleAsset) {
        messenger::get_fa(fa/*caret*/);
    }
}
            "#,
        ),
    ];
    let test_state = fixtures::from_multiple_files_on_tmpfs(test_packages);
    check_diagnostics_on_tmpfs(
        test_state,
        // language=Move
        expect![[r#"
            module std::main {
                use std::messenger;
                public fun main(fa: std::fungible_asset::FungibleAsset) {
                    messenger::get_fa(fa/*caret*/);
                }
            }
        "#]],
    );
}
