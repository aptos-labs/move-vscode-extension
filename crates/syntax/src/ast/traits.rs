// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

mod docs;
pub mod has_item_list;
pub mod has_use_stmts;

use crate::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use crate::ast::{AstChildren, Stmt, support};
use crate::{AstNode, ast};
pub use docs::HoverDocsOwner;
pub use has_item_list::HasItems;
pub use has_use_stmts::HasUseStmts;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::Read;

// pub trait NamedElement: AstNode {
//     fn name(&self) -> Option<ast::Name> {
//         support::child(self.syntax())
//     }
// }

pub(crate) fn into_named_elements(items: Vec<impl Into<ast::NamedElement>>) -> Vec<ast::NamedElement> {
    items.into_iter().map(|it| it.into()).collect()
}

impl ast::Name {
    pub fn as_string(&self) -> String {
        self.ident_token().to_string()
    }
}

pub trait HasStmts: AstNode {
    fn stmts(&self) -> AstChildren<Stmt> {
        support::children(&self.syntax())
    }

    fn let_stmts(&self) -> impl Iterator<Item = ast::LetStmt> {
        self.stmts().filter_map(|it| it.let_stmt())
    }

    fn global_variables(&self) -> Vec<ast::GlobalVariableDecl> {
        self.stmts().filter_map(|it| it.global_variable_decl()).collect()
    }
}

// pub trait GenericElement: AstNode {
//     fn type_param_list(&self) -> Option<ast::TypeParamList> {
//         support::child(&self.syntax())
//     }
//
//     fn type_params(&self) -> Vec<ast::TypeParam> {
//         self.type_param_list()
//             .map(|l| l.type_parameters().collect())
//             .unwrap_or_default()
//     }
// }

pub trait HasAttrs: AstNode {
    fn attrs(&self) -> AstChildren<ast::Attr> {
        support::children(self.syntax())
    }

    fn atom_attrs(&self) -> impl Iterator<Item = String> {
        self.attrs().filter_map(|x| x.as_simple_atom())
    }

    fn has_atom_attr(&self, atom: &str) -> bool {
        self.atom_attrs().contains(atom)
    }
}

// pub trait FieldsOwner: AstNode {
//     #[inline]
//     fn named_field_list(&self) -> Option<ast::NamedFieldList> {
//         support::child(&self.syntax())
//     }
//     #[inline]
//     fn tuple_field_list(&self) -> Option<ast::TupleFieldList> {
//         support::child(&self.syntax())
//     }
//
//     fn named_and_tuple_fields(&self) -> Vec<ast::AnyField> {
//         self.named_fields()
//             .into_iter()
//             .map(|f| f.into())
//             .chain(self.tuple_fields().into_iter().map(|f| f.into()))
//             .collect()
//     }
//
//     fn named_fields(&self) -> Vec<ast::NamedField> {
//         self.named_field_list()
//             .map(|list| list.fields().collect::<Vec<_>>())
//             .unwrap_or_default()
//     }
//
//     fn named_fields_map(&self) -> HashMap<String, ast::NamedField> {
//         self.named_fields()
//             .into_iter()
//             .map(|field| (field.field_name().as_string(), field))
//             .collect()
//     }
//
//     fn tuple_fields(&self) -> Vec<ast::TupleField> {
//         self.tuple_field_list()
//             .map(|list| list.fields().collect::<Vec<_>>())
//             .unwrap_or_default()
//     }
//
//     fn is_fieldless(&self) -> bool {
//         self.named_field_list().is_none() && self.tuple_field_list().is_none()
//     }
// }

// pub trait ReferenceElement: AstNode + fmt::Debug {
//     #[inline]
//     fn cast_into<T: ReferenceElement>(&self) -> Option<T> {
//         T::cast(self.syntax().to_owned())
//     }
//
//     fn reference(&self) -> ast::AnyReferenceElement {
//         self.syntax()
//             .to_owned()
//             .cast::<ast::AnyReferenceElement>()
//             .unwrap()
//     }
// }

pub trait MslOnly: AstNode {}

// pub trait LoopLike: AstNode {
//     fn loop_body_expr(&self) -> Option<ast::BlockOrInlineExpr> {
//         support::child(&self.syntax())
//     }
// }
