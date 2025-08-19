// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast::NamedElement;
use crate::{AstNode, ast};
use std::collections::HashSet;

impl ast::Enum {
    pub fn variants(&self) -> Vec<ast::Variant> {
        self.variant_list()
            .map(|list| list.variants().collect())
            .unwrap_or_default()
    }

    pub fn abilities(&self) -> Vec<ast::Ability> {
        self.ability_list()
            .map(|it| it.abilities().collect())
            .unwrap_or_default()
    }
}

impl ast::Variant {
    pub fn named_fields(&self) -> Vec<ast::NamedField> {
        self.field_list().map(|it| it.named_fields()).unwrap_or_default()
    }

    pub fn tuple_fields(&self) -> Vec<ast::TupleField> {
        self.field_list().map(|it| it.tuple_fields()).unwrap_or_default()
    }

    pub fn enum_(&self) -> ast::Enum {
        let variant_list = self.syntax.parent().unwrap();
        let enum_ = variant_list.parent().unwrap();
        ast::Enum::cast(enum_).unwrap()
    }
}
