use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::ty::Ty;
use crate::types::ty::integer::IntegerKind;
use syntax::ast;

impl<'a, 'db> TypeAstWalker<'a, 'db> {
    pub(super) fn process_predicate_stmt(&mut self, predicate: &ast::SpecPredicateStmt) -> Option<()> {
        let expr = predicate.expr()?;
        self.infer_expr_coerceable_to(&expr, Ty::Bool);
        Some(())
    }

    pub(super) fn process_aborts_if_stmt(&mut self, aborts_if_stmt: &ast::AbortsIfStmt) -> Option<()> {
        let expr = aborts_if_stmt.expr()?;
        self.infer_expr_coerceable_to(&expr, Ty::Bool);
        if let Some(aborts_with) = aborts_if_stmt.aborts_if_with() {
            let with_expr = aborts_with.expr()?;
            self.infer_expr_coerceable_to(&with_expr, Ty::Integer(IntegerKind::Integer));
        }
        Some(())
    }
}
