use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{ast, AstNode};

impl ast::Expr {
    pub fn inference_ctx_owner(&self) -> Option<ast::InferenceCtxOwner> {
        self.syntax().ancestor_strict::<ast::InferenceCtxOwner>()
    }
}
