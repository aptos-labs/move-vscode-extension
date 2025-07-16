// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::init_tracing_for_test;
use expect_test::{Expect, expect};
use ide::inlay_hints::{InlayFieldsToResolve, InlayHintsConfig};
use test_utils::{SourceMark, apply_source_marks, fixtures, remove_marks};

const DISABLED_CONFIG: InlayHintsConfig = InlayHintsConfig {
    render_colons: false,
    type_hints: false,
    parameter_hints: false,
    hide_closure_parameter_hints: false,
    fields_to_resolve: InlayFieldsToResolve::empty(),
};

const TEST_CONFIG: InlayHintsConfig = InlayHintsConfig {
    type_hints: true,
    parameter_hints: true,
    ..DISABLED_CONFIG
};

#[track_caller]
pub(crate) fn check_inlay_hints_with_config(config: &InlayHintsConfig, expect: Expect) {
    init_tracing_for_test();

    let source = stdx::trim_indent(expect.data());
    let trimmed_source = remove_marks(&source, "//^");

    let (analysis, file_id) = fixtures::from_single_file(trimmed_source.clone());

    let inlay_hints = analysis.inlay_hints(config, file_id, None).unwrap();

    let markings = inlay_hints
        .into_iter()
        .map(|it| {
            let text_range = it.range;
            let message = it.label.to_string();
            SourceMark {
                text_range,
                message,
                custom_symbol: None,
            }
        })
        .collect();
    let res = apply_source_marks(trimmed_source.as_str(), markings);
    expect.assert_eq(res.as_str());
}

#[track_caller]
pub(crate) fn check_inlay_hints(expect: Expect) {
    check_inlay_hints_with_config(&TEST_CONFIG, expect);
}

#[test]
fn test_ident_pat_inlay_hints() {
    // language=Move
    check_inlay_hints(expect![[r#"
        module 0x1::m {
            fun main() {
                let a = 1;
                  //^ integer
            }
        }
    "#]]);
}

#[test]
fn test_ident_pat_in_lambda_param() {
    // language=Move
    check_inlay_hints(expect![[r#"
        module 0x1::m {
            fun for_each(v: vector<u8>, f: |u8| u8) {}
            fun main() {
                for_each(vector[], |elem| elem);
                                  //^^^^ u8
            }
        }
    "#]]);
}

#[test]
fn test_item_from_move_stdlib_is_always_local() {
    // language=Move
    check_inlay_hints(expect![[r#"
        module std::string {
            struct String { val: u8 }
            public fun get_s(): String {
                String { val: 1 }
            }
        }
        module 0x1::m {
            use std::string::get_s;
            fun main() {
                let a = get_s();
                  //^ String
            }
        }
    "#]]);
}

#[test]
fn test_item_from_aptos_stdlib_is_always_local() {
    // language=Move
    check_inlay_hints(expect![[r#"
        module aptos_std::string {
            struct String { val: u8 }
            public fun get_s(): String {
                String { val: 1 }
            }
        }
        module 0x1::m {
            use aptos_std::string::get_s;
            fun main() {
                let a = get_s();
                  //^ String
            }
        }
    "#]]);
}

#[test]
fn test_item_from_the_same_package_hints_only_with_name() {
    // language=Move
    check_inlay_hints(expect![[r#"
        module 0x2::price_management {
            struct Price { val: u8 }
            public fun get_s(): Price {
                Price { val: 1 }
            }
        }
        module 0x2::m {
            use 0x2::price_management;
            fun main() {
                let a = price_management::get_s();
                  //^ Price
            }
        }
    "#]]);
}

#[test]
fn test_do_not_show_inlay_hint_for_underscored_params() {
    // language=Move
    check_inlay_hints(expect![[r#"
        module 0x1::m {
            fun for_each(v: vector<u8>, f: |u8| u8) {}
            fun main() {
                for_each(vector[], |_elem| 1);
            }
        }
    "#]]);
}

#[test]
fn test_inlay_hint_for_uninferred_vec() {
    // language=Move
    check_inlay_hints(expect![[r#"
        module 0x1::m {
            fun main() {
                let v = vector[];
                  //^ vector<?>
            }
        }
    "#]]);
}

#[test]
fn test_no_inlay_hint_if_type_is_uninferred() {
    // language=Move
    check_inlay_hints(expect![[r#"
        module 0x1::m {
            fun main() {
                let v = unknown;
            }
        }
    "#]]);
}
