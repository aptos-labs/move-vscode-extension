// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ide_test_utils::diagnostics::check_diagnostics_with_config;
use expect_test::expect;
use ide_diagnostics::config::DiagnosticsConfig;
use std::collections::HashSet;

mod test_replace_with_compound_expr;
mod test_replace_with_index_expr;
mod test_replace_with_method_call;

mod test_missing_fields;
mod test_missing_value_arguments;

mod test_ability_checking;
mod test_check_syntax;
mod test_field_shorthand;
mod test_missing_type_arguments;
mod test_needs_type_annotation;
mod test_redundant_cast;
mod test_type_checking;
mod test_type_checking_fs;
mod test_unresolved_reference;
mod test_unused_acquires;
mod test_unused_imports;
mod test_unused_imports_fix;
mod test_unused_variables;

#[test]
fn test_disable_unresolved_reference() {
    let config = DiagnosticsConfig {
        disabled: {
            let mut codes = HashSet::new();
            codes.insert("unresolved-reference".to_string());
            codes
        },
        ..DiagnosticsConfig::test_sample()
    };
    // language=Move
    check_diagnostics_with_config(
        config,
        expect![[r#"
            module std::main {
                use std::unresolved;
              //^^^^^^^^^^^^^^^^^^^^ warn: Unused use item
                fun main() {
                    1 + true;
                      //^^^^ err: Invalid argument to '+': expected integer type, but found 'bool'
                }
            }
        "#]],
    )
}

#[test]
fn test_enable_only_type_error_disabled_is_ignored() {
    let config = DiagnosticsConfig {
        disabled: {
            let mut codes = HashSet::new();
            codes.insert("type-error".to_string());
            codes
        },
        enable_only: {
            let mut codes = HashSet::new();
            codes.insert("type-error".to_string());
            codes
        },
        ..DiagnosticsConfig::test_sample()
    };
    // language=Move
    check_diagnostics_with_config(
        config,
        expect![[r#"
            module std::main {
                use std::unresolved;
                fun main() {
                    1 + true;
                      //^^^^ err: Invalid argument to '+': expected integer type, but found 'bool'
                }
            }
        "#]],
    )
}
