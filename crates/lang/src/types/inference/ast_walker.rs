use crate::types::inference::InferenceCtx;
use crate::types::lowering::TyLowering;
use crate::types::ty::Ty;
use syntax::ast;
use syntax::ast::{BindingTypeOwner, HasStmts, Pat};
use vfs::FileId;

pub struct TypeAstWalker<'a, 'db> {
    ctx: &'a mut InferenceCtx<'db>,
    file_id: FileId,
}

impl<'a, 'db> TypeAstWalker<'a, 'db> {
    pub fn new(ctx: &'a mut InferenceCtx<'db>, file_id: FileId) -> Self {
        TypeAstWalker { ctx, file_id }
    }

    pub fn collect_parameter_bindings(&mut self, ctx_owner: &ast::InferenceCtxOwner) {
        let bindings = match ctx_owner {
            ast::InferenceCtxOwner::Fun(fun) => fun.params_as_bindings(),
            _ => {
                return;
            }
        };
        let ty_lowering = TyLowering::new(self.ctx.db, self.file_id);
        for binding in bindings {
            let binding_type_owner = binding.type_owner();
            let binding_ty = match binding_type_owner {
                None => Ty::Unknown,
                Some(BindingTypeOwner::Param(fun_param)) => fun_param
                    .type_()
                    .map(|it| ty_lowering.lower_type(it))
                    .unwrap_or(Ty::Unknown),
                // todo:
                _ => Ty::Unknown,
            };
            self.ctx.pat_types.insert(Pat::IdentPat(binding), binding_ty);
        }
    }

    pub fn infer_block_expr(&mut self, block_expr: ast::BlockExpr) -> Ty {
        for stmt in block_expr.stmts() {
            self.process_stmt(stmt);
        }
        // todo: tail expr type
        Ty::Unknown
    }

    fn process_stmt(&mut self, stmt: ast::Stmt) {
        match stmt {
            ast::Stmt::ExprStmt(expr_stmt) => {
                self.infer_expr(expr_stmt.expr());
            }
            _ => {}
        }
    }

    fn infer_expr(&mut self, expr: ast::Expr) -> Ty {
        let expr_ty = match &expr {
            ast::Expr::Literal(lit) => self.infer_literal(lit),
            _ => Ty::Unknown,
        };
        self.ctx.expr_types.insert(expr, expr_ty.clone());
        expr_ty
    }

    fn infer_literal(&self, lit: &ast::Literal) -> Ty {
        if lit.bool_literal_token().is_some() {
            return Ty::Bool;
        }
        Ty::Unknown
    }
}
