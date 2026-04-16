// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ide_test_utils::diagnostics::{check_diagnostics, check_diagnostics_and_fix};
use expect_test::expect;

#[test]
fn test_public_package_triggers_with_fix() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::m {
                public(package) fun f() {}
              //^^^^^^^^^^^^^^^ weak: `public(package)` can be replaced with `package`
            }
        "#]],
        expect![[r#"
            module 0x1::m {
                package fun f() {}
            }
        "#]],
    )
}

#[test]
fn test_other_visibilities_do_not_trigger() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            package fun f_package() {}
            public fun f_public() {}
            friend fun f_friend() {}
            fun f_private() {}
        }
    "#]])
}
