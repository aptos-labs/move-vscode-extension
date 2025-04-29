mod docs;
pub mod has_item_list;
pub mod has_use_stmts;

use crate::ast::{support, AstChildren, Stmt};
use crate::{ast, AstNode};
use std::collections::HashMap;
use std::fmt;
use std::io::Read;

use crate::ast::node_ext::move_syntax_node::MoveSyntaxNodeExt;
pub use docs::HoverDocsOwner;
pub use has_item_list::HasItems;
pub use has_use_stmts::HasUseStmts;

pub trait NamedElement: AstNode {
    fn name(&self) -> Option<ast::Name> {
        support::child(self.syntax())
    }
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
}

pub trait GenericElement: NamedElement {
    fn type_param_list(&self) -> Option<ast::TypeParamList> {
        support::child(&self.syntax())
    }

    fn type_params(&self) -> Vec<ast::TypeParam> {
        self.type_param_list()
            .map(|l| l.type_parameters().collect())
            .unwrap_or_default()
    }
}

pub trait HasAttrs: AstNode {
    fn attrs(&self) -> AstChildren<ast::Attr> {
        support::children(self.syntax())
    }
    fn has_atom_attr(&self, atom: &str) -> bool {
        self.attrs().filter_map(|x| x.as_simple_atom()).any(|x| x == atom)
    }
}

pub trait FieldsOwner: NamedElement {
    #[inline]
    fn named_field_list(&self) -> Option<ast::NamedFieldList> {
        support::child(&self.syntax())
    }
    #[inline]
    fn tuple_field_list(&self) -> Option<ast::TupleFieldList> {
        support::child(&self.syntax())
    }

    fn named_and_tuple_fields(&self) -> Vec<ast::AnyField> {
        self.named_fields()
            .into_iter()
            .map(|f| f.into())
            .chain(self.tuple_fields().into_iter().map(|f| f.into()))
            .collect()
    }

    fn named_fields(&self) -> Vec<ast::NamedField> {
        self.named_field_list()
            .map(|list| list.fields().collect::<Vec<_>>())
            .unwrap_or_default()
    }

    fn named_fields_map(&self) -> HashMap<String, ast::NamedField> {
        self.named_fields()
            .into_iter()
            .map(|field| (field.field_name().as_string(), field))
            .collect()
    }

    fn tuple_fields(&self) -> Vec<ast::TupleField> {
        self.tuple_field_list()
            .map(|list| list.fields().collect::<Vec<_>>())
            .unwrap_or_default()
    }

    fn is_fieldless(&self) -> bool {
        self.named_field_list().is_none() && self.tuple_field_list().is_none()
    }
}

pub trait ReferenceElement: AstNode + fmt::Debug {
    #[inline]
    fn cast_into<T: ReferenceElement>(&self) -> Option<T> {
        T::cast(self.syntax().to_owned())
    }

    fn reference(&self) -> ast::AnyReferenceElement {
        self.syntax()
            .to_owned()
            .cast::<ast::AnyReferenceElement>()
            .unwrap()
    }
}

pub trait MslOnly: AstNode {}

// pub trait LoopLike: AstNode {
//     fn loop_body_expr(&self) -> Option<ast::BlockOrInlineExpr> {
//         support::child(&self.syntax())
//     }
// }
