use crate::types::expectation::Expected;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::ty::integer::IntegerKind;
use crate::types::ty::Ty;
use syntax::ast;
use syntax::files::InFileExt;

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

    pub(super) fn infer_quant_expr(&mut self, quant_expr: &ast::QuantExpr) -> Option<Ty> {
        for quant_binding in quant_expr.quant_bindings() {
            if let Some(ident_pat) = quant_binding.ident_pat() {
                let ty = self.infer_quant_binding_ty(&quant_binding).unwrap_or(Ty::Unknown);
                self.ctx.pat_types.insert(ident_pat.into(), ty);
            }
        }
        if let Some(where_expr) = quant_expr.where_expr().and_then(|it| it.expr()) {
            self.infer_expr_coerceable_to(&where_expr, Ty::Bool);
        }
        self.infer_quant_expr_inner_ty(quant_expr);
        Some(Ty::Bool)
    }

    fn infer_quant_binding_ty(&mut self, quant_binding: &ast::QuantBinding) -> Option<Ty> {
        let ty = if quant_binding.in_token().is_some() {
            let range_expr = quant_binding.expr()?;
            let seq_ty = self.infer_expr(&range_expr, Expected::NoValue).into_ty_seq()?;
            seq_ty.item().refine_for_specs(true)
        } else {
            let type_ = quant_binding.type_()?;
            self.ctx.ty_lowering().lower_type(type_.in_file(self.ctx.file_id))
        };
        Some(ty)
    }

    fn infer_quant_expr_inner_ty(&mut self, quant_expr: &ast::QuantExpr) -> Option<()> {
        match quant_expr {
            ast::QuantExpr::ForallExpr(forall_expr) => {
                let expr = forall_expr.expr()?;
                self.infer_expr_coerceable_to(&expr, Ty::Bool);
            }
            ast::QuantExpr::ExistsExpr(exists_expr) => {
                let expr = exists_expr.expr()?;
                self.infer_expr_coerceable_to(&expr, Ty::Bool);
            }
            ast::QuantExpr::ChooseExpr(_) => (),
        }
        Some(())
    }
}
