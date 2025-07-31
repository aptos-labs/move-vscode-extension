// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::test_inlay_type_hints::check_inlay_hints;
use expect_test::expect;

#[test]
fn test_inlay_range_expr_hint() {
    // language=Move
    check_inlay_hints(expect![[r#"
        module 0x1::m {
            fun main() {
                for (i in 0..10) {
                    i;
                }
            }
        }
    "#]]);
}
