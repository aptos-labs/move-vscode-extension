// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;
use crate::ast::NamedElement;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::NamedField {
    pub fn fields_owner(&self) -> ast::FieldsOwner {
        let named_field_list = self
            .syntax
            .parent_of_type::<ast::NamedFieldList>()
            .expect("`NamedField.named_field_list` is required");
        let fields_owner = named_field_list
            .syntax
            .parent_of_type::<ast::FieldsOwner>()
            .expect("NamedFieldList.fields_owner is required");
        fields_owner
    }

    // invariant checked
    pub fn field_name(&self) -> ast::Name {
        self.name()
            .expect("`name` is required to be present for ast::NamedField")
    }
}
