use crate::ast::operators::UnaryOp;
use crate::{ast, AstNode, SyntaxToken};
use parser::T;

impl ast::PrefixExpr {
    pub fn op_kind(&self) -> Option<UnaryOp> {
        let res = match self.op_token()?.kind() {
            T![*] => UnaryOp::Deref,
            T![!] => UnaryOp::Not,
            // T![-] => UnaryOp::Neg,
            _ => return None,
        };
        Some(res)
    }

    pub fn op_token(&self) -> Option<SyntaxToken> {
        self.syntax().first_child_or_token()?.into_token()
    }
}
