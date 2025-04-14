use crate::ast::support;
use crate::{ast, AstNode};

impl ast::IfExpr {
    pub fn then_branch(&self) -> Option<ast::BlockOrInlineExpr> {
        support::children::<ast::BlockOrInlineExpr>(self.syntax()).nth(0)
    }

    pub fn else_branch(&self) -> Option<ast::BlockOrInlineExpr> {
        // dbg!(&support::children::<ast::BlockOrInlineExpr>(self.syntax()).collect::<Vec<_>>());
        // let else_token = self.else_token()?;
        support::children::<ast::BlockOrInlineExpr>(self.syntax()).nth(1)
    }
}

impl ast::BlockOrInlineExpr {
    pub fn tail_expr(&self) -> Option<ast::Expr> {
        match self {
            ast::BlockOrInlineExpr::InlineExpr(inline_expr) => inline_expr.expr(),
            ast::BlockOrInlineExpr::BlockExpr(block_expr) => block_expr.tail_expr(),
        }
    }
}
