use crate::ast::support;
use crate::{ast, AstNode, IntoNodeOrToken, SyntaxNodeOrToken};

impl ast::IfExpr {
    pub fn condition_expr(&self) -> Option<ast::Expr> {
        self.condition().and_then(|it| it.expr())
    }

    pub fn then_branch(&self) -> Option<ast::BlockOrInlineExpr> {
        support::children::<ast::BlockOrInlineExpr>(self.syntax()).nth(0)
    }

    pub fn else_branch(&self) -> Option<ast::BlockOrInlineExpr> {
        support::children::<ast::BlockOrInlineExpr>(self.syntax()).nth(1)
    }
}

impl ast::BlockOrInlineExpr {
    pub fn tail_node_or_token(&self) -> Option<SyntaxNodeOrToken> {
        match self {
            ast::BlockOrInlineExpr::InlineExpr(inline_expr) => {
                inline_expr.expr().map(|it| it.node_or_token())
            }
            ast::BlockOrInlineExpr::BlockExpr(block_expr) => block_expr
                .tail_expr()
                .map(|it| it.node_or_token())
                .or_else(|| block_expr.r_curly_token().map(|it| it.into())),
        }
    }
}
