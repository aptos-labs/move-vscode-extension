use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{ast, AstNode};

impl ast::Expr {
    pub fn inference_ctx_owner(&self) -> Option<ast::InferenceCtxOwner> {
        self.syntax().ancestor_strict::<ast::InferenceCtxOwner>()
    }
}

impl ast::Expr {
    pub fn is_block_like(&self) -> bool {
        matches!(
            self,
            ast::Expr::IfExpr(_)
                | ast::Expr::LoopExpr(_)
                | ast::Expr::ForExpr(_)
                | ast::Expr::WhileExpr(_)
                | ast::Expr::BlockExpr(_) // | ast::Expr::MatchExpr(_)
        )
    }
}
