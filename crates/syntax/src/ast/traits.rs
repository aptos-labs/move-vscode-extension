pub mod has_item_list;
pub mod has_use_stmts;

use crate::ast::{support, AstChildren, Stmt};
use crate::{ast, AstNode};
use std::io::Read;

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

pub trait GenericItem: AstNode {
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

pub trait FieldsOwner: AstNode {
    #[inline]
    fn named_field_list(&self) -> Option<ast::NamedFieldList> {
        support::child(&self.syntax())
    }
    #[inline]
    fn tuple_field_list(&self) -> Option<ast::TupleFieldList> {
        support::child(&self.syntax())
    }

    fn fields(&self) -> Vec<ast::AnyField> {
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
    fn tuple_fields(&self) -> Vec<ast::TupleField> {
        self.tuple_field_list()
            .map(|list| list.fields().collect::<Vec<_>>())
            .unwrap_or_default()
    }
}

pub trait ReferenceElement: AstNode {
    #[inline]
    fn cast_into<T: ReferenceElement>(&self) -> Option<T> {
        T::cast(self.syntax().to_owned())
    }
}

pub trait MslOnly: AstNode {}
