// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;
use crate::ast::FieldsOwner;

impl ast::Struct {
    pub fn named_fields(&self) -> Vec<ast::NamedField> {
        self.field_list().map(|it| it.named_fields()).unwrap_or_default()
    }

    pub fn tuple_fields(&self) -> Vec<ast::TupleField> {
        self.field_list().map(|it| it.tuple_fields()).unwrap_or_default()
    }

    pub fn is_tuple_struct(&self) -> bool {
        self.field_list().and_then(|it| it.tuple_field_list()).is_some()
    }

    pub fn wrapped_lambda_type(&self) -> Option<ast::LambdaType> {
        let mut tuple_fields = self.tuple_fields();
        match tuple_fields.len() {
            1 => {
                let tuple_field = tuple_fields.pop().unwrap();
                let lambda_type = tuple_field.type_().and_then(|it| it.lambda_type());
                lambda_type
            }
            _ => None,
        }
    }
}
