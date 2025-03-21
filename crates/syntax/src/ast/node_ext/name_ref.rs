use crate::ast::node_ext::text_of_first_token;
use crate::token_text::TokenText;
use crate::{ast, AstNode};
use parser::SyntaxKind;

impl ast::NameRef {
    pub fn text(&self) -> TokenText<'_> {
        text_of_first_token(self.syntax())
    }

    pub fn as_tuple_field(&self) -> Option<usize> {
        self.text().parse().ok()
    }

    pub fn token_kind(&self) -> SyntaxKind {
        self.syntax()
            .first_token()
            .map_or(SyntaxKind::ERROR, |it| it.kind())
    }

    pub fn as_string(&self) -> String {
        self.ident_token().to_string()
    }
}
