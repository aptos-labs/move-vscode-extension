use crate::{
    ast::{self, make},
    AstNode, SyntaxToken,
};
use parser::SyntaxKind;

use super::SyntaxFactory;

impl SyntaxFactory {
    pub fn name(&self, name: &str) -> ast::Name {
        make::name(name).clone_for_update()
    }

    pub fn name_ref(&self, name: &str) -> ast::NameRef {
        make::name_ref(name).clone_for_update()
    }

    pub fn token(&self, kind: SyntaxKind) -> SyntaxToken {
        make::token(kind)
    }

    pub fn whitespace(&self, text: &str) -> SyntaxToken {
        make::tokens::whitespace(text)
    }
}
