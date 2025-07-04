// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::FieldList {
    pub fn named_fields(&self) -> Vec<ast::NamedField> {
        self.clone()
            .named_field_list()
            .map(|list| list.fields().collect::<Vec<_>>())
            .unwrap_or_default()
    }

    pub fn tuple_fields(&self) -> Vec<ast::TupleField> {
        self.clone()
            .tuple_field_list()
            .map(|list| list.fields().collect::<Vec<_>>())
            .unwrap_or_default()
    }
}
