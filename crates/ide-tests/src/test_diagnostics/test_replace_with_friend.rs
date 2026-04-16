// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ide_test_utils::diagnostics::{check_diagnostics, check_diagnostics_and_fix};
use expect_test::expect;

#[test]
fn test_public_friend_triggers_with_fix() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::m {
                public(friend) fun f() {}
              //^^^^^^^^^^^^^^ weak: `public(friend)` can be replaced with `friend`
            }
        "#]],
        expect![[r#"
            module 0x1::m {
                friend fun f() {}
            }
        "#]],
    )
}

#[test]
fn test_other_visibilities_do_not_trigger() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            friend fun f_friend() {}
            public fun f_public() {}
            package fun f_public_package() {}
            fun f_private() {}
        }
    "#]])
}
