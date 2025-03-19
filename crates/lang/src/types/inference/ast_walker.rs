use crate::types::inference::InferenceCtx;
use crate::types::patterns::{anonymous_pat_ty_var, collect_bindings, BindingMode};
use crate::types::ty::Ty;
use syntax::ast;
use syntax::ast::{BindingTypeOwner, HasStmts, Pat};

pub struct TypeAstWalker<'a, 'db> {
    pub ctx: &'a mut InferenceCtx<'db>,
}

impl<'a, 'db> TypeAstWalker<'a, 'db> {
    pub fn new(ctx: &'a mut InferenceCtx<'db>) -> Self {
        TypeAstWalker { ctx }
    }

    pub fn collect_parameter_bindings(&mut self, ctx_owner: &ast::InferenceCtxOwner) {
        let bindings = match ctx_owner {
            ast::InferenceCtxOwner::Fun(fun) => fun.params_as_bindings(),
            _ => {
                return;
            }
        };
        for binding in bindings {
            let binding_ty = {
                let binding_type_owner = binding.type_owner();
                let ty_lowering = self.ctx.ty_lowering();
                match binding_type_owner {
                    Some(BindingTypeOwner::Param(fun_param)) => fun_param
                        .type_()
                        .map(|it| ty_lowering.lower_type(it))
                        .unwrap_or(Ty::Unknown),
                    _ => continue,
                }
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
            ast::Stmt::LetStmt(let_stmt) => {
                let explicit_ty = let_stmt.type_().map(|it| self.ctx.ty_lowering().lower_type(it));
                let pat = let_stmt.pat();
                let initializer_ty = match let_stmt.initializer() {
                    None => pat
                        .clone()
                        .map(|it| anonymous_pat_ty_var(&mut self.ctx.ty_var_counter, &it))
                        .unwrap_or(Ty::Unknown),
                    Some(initializer_expr) => {
                        let initializer_ty = self.infer_expr(initializer_expr);
                        explicit_ty.unwrap_or(initializer_ty)
                    }
                };
                if let Some(pat) = pat {
                    collect_bindings(
                        self,
                        pat,
                        self.ctx.resolve_vars_if_possible(initializer_ty),
                        BindingMode::BindByValue,
                    );
                }
            }
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
