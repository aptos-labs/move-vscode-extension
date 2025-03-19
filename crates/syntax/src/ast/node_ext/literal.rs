use crate::{ast, SyntaxToken};

impl ast::Literal {
    pub fn bool_literal_token(&self) -> Option<SyntaxToken> {
        self.false_token().or(self.true_token())
    }
}
