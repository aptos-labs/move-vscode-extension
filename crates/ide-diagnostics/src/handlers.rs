// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

pub(crate) mod call_params;
mod can_be_replaced_with_compound_expr;
mod can_be_replaced_with_index_expr;
mod can_be_replaced_with_method_call;
pub(crate) mod error_const_docs;
pub(crate) mod missing_fields;
mod redundant_cast;
mod type_checking;
mod unresolved_reference;
pub(crate) mod unused_variables;

pub(crate) use can_be_replaced_with_compound_expr::can_be_replaced_with_compound_expr;
pub(crate) use can_be_replaced_with_index_expr::can_be_replaced_with_index_expr;
pub(crate) use can_be_replaced_with_method_call::can_be_replaced_with_method_call;
pub(crate) use redundant_cast::redundant_integer_cast;
pub(crate) use type_checking::{recursive_struct_check, type_check};
pub(crate) use unresolved_reference::find_unresolved_references;

use ide_db::assists::{Assist, AssistId};
use ide_db::label::Label;
use ide_db::source_change::SourceChange;
use syntax::TextRange;

fn fix(id: &'static str, label: &str, source_change: SourceChange, target: TextRange) -> Assist {
    assert!(!id.contains(' '));
    Assist {
        id: AssistId::quick_fix(id),
        label: Label::new(label.to_owned()),
        target,
        source_change: Some(source_change),
        command: None,
    }
}
