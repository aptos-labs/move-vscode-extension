// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::init_tracing_for_test;
use syntax::files::FilePosition;
use test_utils::{fixtures, get_marked_position_offset_with_data};

mod test_call_expr_types;
mod test_function_values;
mod test_lambda_param_types;
mod test_my_types;
mod test_types_expression_types;

pub fn check_expr_type(source: &str) {
    init_tracing_for_test();

    let (analysis, file_id) = fixtures::from_single_file(source.to_string());

    let (ref_offset, data) = get_marked_position_offset_with_data(&source, "//^");
    let position = FilePosition { file_id, offset: ref_offset };

    let opt_ty = analysis.expr_type_info(position).unwrap();
    let expr_ty = opt_ty.expect("could not find an expr / outside inference context");

    assert_eq!(expr_ty, data);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr_type() {
        // language=Move
        check_expr_type(
            r#"
module 0x1::m {
    fun call<T>(val: T): T {
        val
    }
    fun main() {
        (call(1u8));
      //^ u8
    }
}
"#,
        );
    }
}
