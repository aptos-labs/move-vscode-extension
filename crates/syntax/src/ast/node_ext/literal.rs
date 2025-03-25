use crate::{ast, AstNode, AstToken, SyntaxToken};
use parser::SyntaxKind::ATTR;
use parser::T;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LiteralKind {
    // String(ast::String),
    ByteString(ast::ByteString),
    // CString(ast::CString),
    IntNumber(ast::IntNumber),
    Address(ast::AddressLit),
    // FloatNumber(ast::FloatNumber),
    // Char(ast::Char),
    // Byte(ast::Byte),
    Bool(bool),
    Invalid,
}

impl ast::Literal {
    pub fn token(&self) -> SyntaxToken {
        self.syntax()
            .children_with_tokens()
            .find(|e| e.kind() != ATTR && !e.kind().is_trivia())
            .and_then(|e| e.into_token())
            .unwrap()
    }

    pub fn kind(&self) -> LiteralKind {
        if let Some(address_lit) = self.address_lit() {
            return LiteralKind::Address(address_lit);
        }

        let token = self.token();

        if let Some(t) = ast::IntNumber::cast(token.clone()) {
            return LiteralKind::IntNumber(t);
        }

        // if let Some(t) = ast::FloatNumber::cast(token.clone()) {
        //     return LiteralKind::FloatNumber(t);
        // }
        // if let Some(t) = ast::String::cast(token.clone()) {
        //     return LiteralKind::String(t);
        // }
        if let Some(t) = ast::ByteString::cast(token.clone()) {
            return LiteralKind::ByteString(t);
        }
        // if let Some(t) = ast::CString::cast(token.clone()) {
        //     return LiteralKind::CString(t);
        // }
        // if let Some(t) = ast::Char::cast(token.clone()) {
        //     return LiteralKind::Char(t);
        // }
        // if let Some(t) = ast::Byte::cast(token.clone()) {
        //     return LiteralKind::Byte(t);
        // }

        match token.kind() {
            T![true] => LiteralKind::Bool(true),
            T![false] => LiteralKind::Bool(false),
            _ => LiteralKind::Invalid,
        }
    }
}

// impl ast::Literal {
//     pub fn bool_literal_token(&self) -> Option<SyntaxToken> {
//         self.false_token().or(self.true_token())
//     }
// }
