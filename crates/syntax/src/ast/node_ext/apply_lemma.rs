use crate::ast;

impl ast::ApplyLemma {
    #[inline]
    pub fn to_any_call_expr(&self) -> ast::AnyCallExpr {
        ast::AnyCallExpr::ApplyLemma(self.clone())
    }
}
