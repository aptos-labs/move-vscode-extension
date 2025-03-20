use crate::types::expectation::Expectation;
use crate::types::inference::InferenceCtx;
use crate::types::patterns::{anonymous_pat_ty_var, collect_bindings, BindingMode};
use crate::types::ty::reference::autoborrow;
use crate::types::ty::ty_callable::TyCallable;
use crate::types::ty::{IntegerKind, Ty};
use parser::SyntaxKind;
use std::ops::Deref;
use syntax::ast::{BindingTypeOwner, HasStmts, Pat};
use syntax::{ast, AstNode};

pub struct TypeAstWalker<'a, 'db> {
    pub ctx: &'a mut InferenceCtx<'db>,
}

impl<'a, 'db> TypeAstWalker<'a, 'db> {
    pub fn new(ctx: &'a mut InferenceCtx<'db>) -> Self {
        TypeAstWalker { ctx }
    }

    pub fn walk(&mut self, ctx_owner: ast::InferenceCtxOwner) {
        self.collect_parameter_bindings(&ctx_owner);

        match ctx_owner {
            ast::InferenceCtxOwner::Fun(fun) => {
                if let Some(fun_block_expr) = fun.body() {
                    self.infer_block_expr(fun_block_expr);
                }
            }
            _ => {}
        }
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
                        let initializer_ty = self.infer_expr(&initializer_expr, Expectation::NoValue);
                        explicit_ty.clone().unwrap_or(initializer_ty)
                    }
                };
                if let Some(pat) = pat {
                    let pat_ty =
                        explicit_ty.unwrap_or(self.ctx.resolve_vars_if_possible(initializer_ty));
                    collect_bindings(self, pat, pat_ty, BindingMode::BindByValue);
                }
            }
            ast::Stmt::ExprStmt(expr_stmt) => {
                self.infer_expr(&expr_stmt.expr(), Expectation::NoValue);
            }
            _ => {}
        }
    }

    fn infer_expr(&mut self, expr: &ast::Expr, _expected: Expectation) -> Ty {
        let expr_ty = match expr {
            ast::Expr::CallExpr(call_expr) => self.infer_call_expr(call_expr, Expectation::empty()),
            ast::Expr::ParenExpr(paren_expr) => paren_expr
                .expr()
                .map(|it| self.infer_expr(&it, Expectation::NoValue))
                .unwrap_or(Ty::Unknown),
            ast::Expr::Literal(lit) => self.infer_literal(lit),
            _ => Ty::Unknown,
        };
        self.ctx.expr_types.insert(expr.to_owned(), expr_ty.clone());
        expr_ty
    }

    fn infer_call_expr(&mut self, call_expr: &ast::CallExpr, expected: Expectation) -> Ty {
        let Some(callable_ty) = self.instantiate_callable_ty(call_expr) else {
            return Ty::Unknown;
        };

        let expected_arg_tys = self.infer_expected_arg_tys(&callable_ty, expected);
        let args = call_expr
            .args()
            .into_iter()
            .map(|expr| CallArg::Arg { expr })
            .collect();
        self.coerce_argument_types(args, callable_ty.param_types, expected_arg_tys);

        callable_ty.ret_type.deref().to_owned()
    }

    fn instantiate_callable_ty(&self, call_expr: &ast::CallExpr) -> Option<TyCallable> {
        let path = call_expr.path();
        let named_item = self.ctx.resolve_path(path.clone());
        let callable_ty = if let Some(named_item) = named_item {
            let item_kind = named_item.value.syntax().kind();
            match item_kind {
                SyntaxKind::FUN => {
                    let generic_item = named_item.map(|it| it.cast::<ast::Fun>().unwrap().into());
                    let Some(call_ty) = self.ctx.instantiate_path(path, generic_item).ty_callable()
                    else {
                        return None;
                    };
                    call_ty
                }
                _ => TyCallable::fake(call_expr.args().len()),
            }
        } else {
            TyCallable::fake(call_expr.args().len())
        };
        Some(callable_ty)
    }

    fn infer_expected_arg_tys(&mut self, ty_callable: &TyCallable, expected: Expectation) -> Vec<Ty> {
        let Some(expected_ret_ty) = expected.ty(self.ctx) else {
            return vec![];
        };
        let declared_ret_ty = self
            .ctx
            .resolve_vars_if_possible(ty_callable.ret_type.deref().to_owned());

        // unify return types and check if they are compatible
        let combined = self.ctx.combine_types(expected_ret_ty, declared_ret_ty);
        match combined {
            Ok(()) => ty_callable
                .param_types
                .iter()
                .map(|t| self.ctx.resolve_vars_if_possible(t.clone()))
                .collect(),
            Err(_) => vec![],
        }

        // self.ctx.freeze(|| {
        // })
    }

    pub fn coerce_argument_types(
        &mut self,
        args: Vec<CallArg>,
        declared_tys: Vec<Ty>,
        expected_tys: Vec<Ty>,
    ) {
        for (i, arg) in args.into_iter().enumerate() {
            let declared_ty = declared_tys.get(i).unwrap_or(&Ty::Unknown).to_owned();
            let expected_ty = self
                .ctx
                .resolve_vars_if_possible(expected_tys.get(i).unwrap_or(&declared_ty).to_owned());
            match arg {
                CallArg::Self_ { self_ty } => {
                    let actual_self_ty = autoborrow(self_ty, &expected_ty)
                        .expect("method call won't be resolved if autoborrow fails");
                    let _ = self.ctx.combine_types(actual_self_ty, expected_ty);
                }
                CallArg::Arg { expr } => {
                    let arg_expr_ty =
                        self.infer_expr(&expr, Expectation::ExpectType(expected_ty.clone()));
                    self.ctx.coerce_types(expr.syntax(), arg_expr_ty, expected_ty);
                }
            }
        }
    }

    fn infer_literal(&self, lit: &ast::Literal) -> Ty {
        if lit.bool_literal_token().is_some() {
            return Ty::Bool;
        }
        if let Some(int_token) = lit.int_number_token() {
            return Ty::Integer(IntegerKind::from_literal(int_token.text()));
        }
        Ty::Unknown
    }
}

enum CallArg {
    Self_ { self_ty: Ty },
    Arg { expr: ast::Expr },
}
