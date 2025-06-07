use crate::ast::node_ext::text_of_first_token;
use crate::{ast, AstNode, TokenText};
use rowan::{GreenNodeData, GreenTokenData, NodeOrToken};
use std::borrow::Cow;

impl ast::NameRef {
    pub fn text(&self) -> TokenText<'_> {
        text_of_first_token(self.syntax())
    }

    // pub fn as_tuple_field(&self) -> Option<usize> {
    //     self.index_string().and_then(|it| it.parse().ok())
    // }

    // pub fn token_kind(&self) -> SyntaxKind {
    //     self.syntax()
    //         .first_token()
    //         .map_or(SyntaxKind::ERROR, |it| it.kind())
    // }

    pub fn as_string(&self) -> String {
        if let Some(ident_token) = self.ident_token() {
            ident_token.text().to_string()
        } else if let Some(int_number_token) = self.int_number_token() {
            int_number_token.text().to_string()
        } else {
            // can't be "" in the current implementation,
            // it's either non-empty string or int number
            "".to_string()
        }
    }

    // pub fn index_string(&self) -> Option<String> {
    //     self.int_number_token().map(|it| it.text().to_string())
    // }
}
