mod address_ref;
pub mod attr;
pub mod bin_expr;
mod block_expr;
mod call_expr;
mod enum_;
mod expr;
pub mod fun;
mod ident_pat;
pub mod index_expr;
pub mod literal;
mod method_call_expr;
mod method_or_path;
mod module;
pub mod move_syntax_node;
pub mod name_ref;
mod pat;
pub mod path;
pub mod prefix_expr;
mod ref_type;
mod schema;
mod source_file;
mod struct_or_enum;
mod struct_pat_field;
pub mod syntax_node;
pub mod type_;
pub mod visibility;
mod field_ref;
mod borrow_expr;
mod assert_macro_expr;
mod vector_lit_expr;

use crate::token_text::TokenText;
use crate::{ast, AstNode, AstToken, SyntaxNode};
use parser::SyntaxKind;
use rowan::{GreenNodeData, GreenTokenData, NodeOrToken};
use std::borrow::Cow;

impl ast::Name {
    pub fn text(&self) -> TokenText<'_> {
        text_of_first_token(self.syntax())
    }
}

fn text_of_first_token(node: &SyntaxNode) -> TokenText<'_> {
    fn first_token(green_ref: &GreenNodeData) -> &GreenTokenData {
        green_ref
            .children()
            .next()
            .and_then(NodeOrToken::into_token)
            .unwrap()
    }

    match node.green() {
        Cow::Borrowed(green_ref) => TokenText::borrowed(first_token(green_ref).text()),
        Cow::Owned(green) => TokenText::owned(first_token(&green).to_owned()),
    }
}

impl ast::PathSegment {
    pub fn parent_path(&self) -> ast::Path {
        self.syntax()
            .parent()
            .and_then(ast::Path::cast)
            .expect("segments are always nested in paths")
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum NameLike {
    NameRef(ast::NameRef),
    Name(ast::Name),
}

impl NameLike {
    pub fn as_name_ref(&self) -> Option<&ast::NameRef> {
        match self {
            NameLike::NameRef(name_ref) => Some(name_ref),
            _ => None,
        }
    }
    pub fn to_name_ref(self) -> Option<ast::NameRef> {
        match self {
            NameLike::NameRef(name_ref) => Some(name_ref),
            _ => None,
        }
    }
    pub fn as_name(&self) -> Option<&ast::Name> {
        match self {
            NameLike::Name(name) => Some(name),
            _ => None,
        }
    }
    pub fn to_name(self) -> Option<ast::Name> {
        match self {
            NameLike::Name(name) => Some(name),
            _ => None,
        }
    }
    // pub fn text(&self) -> TokenText<'_> {
    //     match self {
    //         NameLike::NameRef(name_ref) => name_ref.text(),
    //         NameLike::Name(name) => name.text(),
    //     }
    // }
}

impl ast::AstNode for NameLike {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, SyntaxKind::NAME | SyntaxKind::NAME_REF)
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        let res = match syntax.kind() {
            SyntaxKind::NAME => NameLike::Name(ast::Name { syntax }),
            SyntaxKind::NAME_REF => NameLike::NameRef(ast::NameRef { syntax }),
            _ => return None,
        };
        Some(res)
    }
    fn syntax(&self) -> &SyntaxNode {
        match self {
            NameLike::NameRef(it) => it.syntax(),
            NameLike::Name(it) => it.syntax(),
        }
    }
}

const _: () = {
    use ast::{Name, NameRef};
    stdx::impl_from!(NameRef, Name for NameLike);
};
