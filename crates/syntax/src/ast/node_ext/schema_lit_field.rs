// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{AstNode, ast};

impl ast::SchemaLitField {
    pub fn schema_lit(&self) -> Option<ast::SchemaLit> {
        self.syntax().ancestor_of_type::<ast::SchemaLit>(true)
    }

    pub fn field_name(&self) -> Option<String> {
        if let Some(name_ref) = self.name_ref() {
            return Some(name_ref.as_string());
        }
        let path = self.expr()?.path_expr()?.path();
        if path.coloncolon_token().is_none() {
            return path.reference_name();
        }
        None
    }
}
