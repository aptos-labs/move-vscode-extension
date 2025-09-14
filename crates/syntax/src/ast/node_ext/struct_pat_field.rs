// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{AstNode, ast};

impl ast::StructPatField {
    pub fn struct_pat(&self) -> ast::StructPat {
        self.syntax()
            .ancestor_strict::<ast::StructPat>()
            .expect("required by parser")
    }

    pub fn field_kind(&self) -> PatFieldKind {
        if let Some(name_ref) = self.name_ref() {
            // at least `NAME_REF:` is available
            return PatFieldKind::Full { name_ref, pat: self.pat() };
        }
        if let Some(ident_pat) = self.ident_pat() {
            return PatFieldKind::Shorthand { ident_pat };
        }
        if let Some(rest_pat) = self.rest_pat() {
            return PatFieldKind::Rest;
        }
        PatFieldKind::Invalid
    }

    pub fn is_shorthand(&self) -> bool {
        matches!(self.field_kind(), PatFieldKind::Shorthand { .. })
    }

    pub fn for_field_name_ref(field_name: &ast::NameRef) -> Option<ast::StructPatField> {
        field_name.syntax().parent_of_type::<ast::StructPatField>()
    }

    pub fn for_field_name(field_name: &ast::Name) -> Option<ast::StructPatField> {
        let ident_pat = field_name.syntax.parent_of_type::<ast::IdentPat>()?;
        let pat_field = ident_pat.syntax.parent_of_type::<ast::StructPatField>()?;
        Some(pat_field)
    }

    pub fn field_name(&self) -> Option<String> {
        match self.field_kind() {
            PatFieldKind::Full { name_ref, .. } => Some(name_ref.as_string()),
            PatFieldKind::Shorthand { ident_pat } => Some(ident_pat.name()?.as_string()),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum PatFieldKind {
    Full {
        name_ref: ast::NameRef,
        pat: Option<ast::Pat>,
    },
    Shorthand {
        ident_pat: ast::IdentPat,
    },
    Rest,
    Invalid,
}
