use crate::files::InFileExt;
use crate::nameres::name_resolution::get_field_lookup_resolve_variants;
use crate::nameres::path_resolution::get_method_resolve_variants;
use crate::nameres::scope::{ScopeEntryListExt, VecExt};
use crate::types::expectation::Expected;
use crate::types::fold::TypeFoldable;
use crate::types::inference::InferenceCtx;
use crate::types::patterns::{anonymous_pat_ty_var, collect_bindings, BindingMode};
use crate::types::substitution::ApplySubstitution;
use crate::types::ty::integer::IntegerKind;
use crate::types::ty::reference::autoborrow;
use crate::types::ty::ty_callable::TyCallable;
use crate::types::ty::Ty;
use std::iter;
use std::ops::Deref;
use syntax::ast::{BindingTypeOwner, HasStmts, NamedElement, Pat};
use syntax::{ast, AstNode, IntoNodeOrToken};

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
                    self.infer_block_expr(&fun_block_expr, Expected::NoValue);
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

    pub fn infer_block_expr(&mut self, block_expr: &ast::BlockExpr, expected: Expected) -> Ty {
        for stmt in block_expr.stmts() {
            self.process_stmt(stmt);
        }

        let tail_expr = block_expr.tail_expr();
        let opt_expected_ty = expected.ty(self.ctx);
        match tail_expr {
            None => {
                if let Some(expected_ty) = opt_expected_ty {
                    let error_target = block_expr
                        .r_curly_token()
                        .map(|it| it.into())
                        .unwrap_or(block_expr.node_or_token());
                    self.ctx.coerce_types(error_target, Ty::Unit, expected_ty);
                }
                Ty::Unit
            }
            Some(tail_expr) => {
                if let Some(expected_ty) = opt_expected_ty {
                    return self.infer_expr_coerce_to(&tail_expr, expected_ty);
                }
                self.infer_expr(&tail_expr, Expected::NoValue)
            }
        }
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
                        let initializer_ty = self.infer_expr(&initializer_expr, Expected::NoValue);
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
                self.infer_expr(&expr_stmt.expr(), Expected::NoValue);
            }
            _ => {}
        }
    }

    fn infer_expr_coerce_to(&mut self, expr: &ast::Expr, expected_ty: Ty) -> Ty {
        let actual_ty = self.infer_expr(expr, Expected::ExpectType(expected_ty.clone()));
        let no_type_error =
            self.ctx
                .coerce_types(expr.node_or_token(), actual_ty.clone(), expected_ty.clone());
        if no_type_error {
            expected_ty
        } else {
            actual_ty
        }
    }

    fn infer_expr(&mut self, expr: &ast::Expr, expected: Expected) -> Ty {
        let expr_ty = match expr {
            ast::Expr::PathExpr(path_expr) => self.infer_path_expr(path_expr, Expected::NoValue),
            ast::Expr::CallExpr(call_expr) => self.infer_call_expr(call_expr, Expected::NoValue),
            ast::Expr::MethodCallExpr(method_call_expr) => {
                self.infer_method_call_expr(method_call_expr, Expected::NoValue)
            }
            ast::Expr::DotExpr(dot_expr) => self.infer_dot_expr(dot_expr, Expected::NoValue),
            ast::Expr::ParenExpr(paren_expr) => paren_expr
                .expr()
                .map(|it| self.infer_expr(&it, Expected::NoValue))
                .unwrap_or(Ty::Unknown),

            ast::Expr::BorrowExpr(borrow_expr) => self.infer_borrow_expr(borrow_expr),
            ast::Expr::DerefExpr(deref_expr) => self.infer_deref_expr(deref_expr),
            ast::Expr::IndexExpr(index_expr) => self.infer_index_expr(index_expr),
            ast::Expr::ResourceExpr(res_expr) => self.infer_resource_expr(res_expr),

            ast::Expr::AbortExpr(abort_expr) => {
                if let Some(inner_expr) = abort_expr.expr() {
                    self.infer_expr(&inner_expr, Expected::NoValue);
                }
                Ty::Never
            }
            ast::Expr::BlockExpr(block_expr) => self.infer_block_expr(block_expr, Expected::NoValue),
            ast::Expr::BinExpr(bin_expr) => self.infer_bin_expr(bin_expr),
            ast::Expr::BangExpr(bang_expr) => bang_expr
                .expr()
                .map(|it| {
                    self.infer_expr(&it, Expected::ExpectType(Ty::Bool));
                    Ty::Bool
                })
                .unwrap_or(Ty::Unknown),
            ast::Expr::Literal(lit) => self.infer_literal(lit),
        };
        self.ctx.expr_types.insert(expr.to_owned(), expr_ty.clone());
        expr_ty
    }

    fn infer_path_expr(&mut self, path_expr: &ast::PathExpr, expected: Expected) -> Ty {
        use syntax::SyntaxKind::*;

        let expected_ty = expected.ty(self.ctx);
        let Some(named_element) = self.ctx.resolve_path_cached(path_expr.path(), expected_ty) else {
            return Ty::Unknown;
        };

        let ty_lowering = self.ctx.ty_lowering();
        match named_element.kind() {
            IDENT_PAT => {
                let ident_pat = named_element.cast::<ast::IdentPat>().unwrap().value;
                self.ctx.get_binding_type(ident_pat)
            }
            CONST => {
                let const_type = named_element.cast::<ast::Const>().unwrap().value.type_();
                const_type
                    .map(|type_| ty_lowering.lower_type(type_))
                    .unwrap_or(Ty::Unknown)
            }

            MODULE => Ty::Unknown,
            // todo: return TyCallable when "function values" feature is implemented
            FUN | SPEC_FUN | SPEC_INLINE_FUN => Ty::Unknown,

            _ => Ty::Unknown,
        }
    }

    fn infer_dot_expr(&mut self, dot_expr: &ast::DotExpr, expected: Expected) -> Ty {
        let self_ty = self.infer_expr(&dot_expr.receiver_expr(), Expected::NoValue);
        let self_ty = self.ctx.resolve_vars_if_possible(self_ty);

        let Some(ty_adt) = self_ty.deref().into_ty_adt() else {
            return Ty::Unknown;
        };
        let adt_item = ty_adt
            .adt_item
            .cast_into::<ast::StructOrEnum>(self.ctx.db.upcast())
            .unwrap();
        let Some(reference_name) = dot_expr
            .field_ref()
            .and_then(|it| it.name_ref().map(|it| it.as_string()))
        else {
            return Ty::Unknown;
        };

        // todo: cannot resolve in outside of declared module
        // todo: tuple index fields

        let named_field = get_field_lookup_resolve_variants(adt_item.value)
            .into_iter()
            .filter(|it| it.name().unwrap().as_string() == reference_name)
            .collect::<Vec<_>>()
            .single_or_none();

        let ty_lowering = self.ctx.ty_lowering();
        let Some(named_field_type) = named_field.and_then(|it| it.type_()) else {
            return Ty::Unknown;
        };

        ty_lowering
            .lower_type(named_field_type)
            .substitute(ty_adt.substitution)
    }

    fn infer_method_call_expr(
        &mut self,
        method_call_expr: &ast::MethodCallExpr,
        expected: Expected,
    ) -> Ty {
        let self_ty = self.infer_expr(&method_call_expr.receiver_expr(), Expected::NoValue);
        let self_ty = self.ctx.resolve_vars_if_possible(self_ty);

        let method_entry = get_method_resolve_variants(self.ctx.db, &self_ty)
            .filter_by_name(method_call_expr.reference_name())
            .filter_by_visibility(self.ctx.db, &method_call_expr.clone().in_file(self.ctx.file_id))
            .single_or_none();
        self.ctx
            .resolved_method_calls
            .insert(method_call_expr.to_owned(), method_entry.clone());

        let resolved_method =
            method_entry.and_then(|it| it.node_loc.cast_into::<ast::Fun>(self.ctx.db.upcast()));
        let method_ty = match resolved_method {
            Some(method) => self
                .ctx
                .instantiate_path_for_fun(method_call_expr.to_owned().into(), method),
            None => {
                // add 1 for `self` parameter
                TyCallable::fake(1 + method_call_expr.args().len())
            }
        };
        let method_ty = method_ty.deep_fold_with(self.ctx.var_resolver());

        let expected_arg_tys = self.infer_expected_call_arg_tys(&method_ty, expected);
        let args = iter::once(CallArg::Self_ { self_ty })
            .chain(
                method_call_expr
                    .args()
                    .into_iter()
                    .map(|arg_expr| CallArg::Arg { expr: arg_expr }),
            )
            .collect();
        self.coerce_call_arg_types(args, method_ty.param_types, expected_arg_tys);

        method_ty.ret_type.deref().to_owned()
    }

    fn infer_call_expr(&mut self, call_expr: &ast::CallExpr, expected: Expected) -> Ty {
        let call_ty = self.ctx.instantiate_call_expr_path(call_expr);

        let expected_arg_tys = self.infer_expected_call_arg_tys(&call_ty, expected);
        let args = call_expr
            .args()
            .into_iter()
            .map(|expr| CallArg::Arg { expr })
            .collect();
        self.coerce_call_arg_types(args, call_ty.param_types, expected_arg_tys);

        call_ty.ret_type.deref().to_owned()
    }

    fn infer_index_expr(&mut self, index_expr: &ast::IndexExpr) -> Ty {
        Ty::Unknown
    }

    fn infer_borrow_expr(&mut self, borrow_expr: &ast::BorrowExpr) -> Ty {
        Ty::Unknown
    }

    fn infer_deref_expr(&mut self, deref_expr: &ast::DerefExpr) -> Ty {
        Ty::Unknown
    }

    fn infer_resource_expr(&mut self, resource_expr: &ast::ResourceExpr) -> Ty {
        Ty::Unknown
    }

    fn infer_bin_expr(&mut self, bin_expr: &ast::BinExpr) -> Ty {
        let Some((lhs, (_, op_kind), rhs)) = bin_expr.unpack() else {
            return Ty::Unknown;
        };
        match op_kind {
            ast::BinaryOp::ArithOp(op) => self.infer_arith_binary_expr(lhs, op, rhs, false),
            _ => Ty::Unknown,
        }
    }

    fn infer_arith_binary_expr(
        &mut self,
        lhs: ast::Expr,
        _op: ast::ArithOp,
        rhs: ast::Expr,
        is_compound: bool,
    ) -> Ty {
        let mut is_error = false;
        let left_ty = self.infer_expr(&lhs, Expected::NoValue);
        if !left_ty.supports_arithm_op() {
            // todo: report error
            is_error = true;
        }
        let right_ty = self.infer_expr(&rhs, Expected::ExpectType(left_ty.clone()));
        if !right_ty.supports_arithm_op() {
            // todo: report error
            is_error = true;
        }
        if !is_error {
            let combined = self.ctx.combine_types(left_ty.clone(), right_ty);
            if combined.is_err() {
                // todo: report error
                is_error = true;
            }
        }

        if is_error {
            Ty::Unknown
        } else {
            if is_compound {
                Ty::Unit
            } else {
                left_ty
            }
        }
    }

    fn infer_literal(&self, literal: &ast::Literal) -> Ty {
        match literal.kind() {
            ast::LiteralKind::Bool(_) => Ty::Bool,
            ast::LiteralKind::IntNumber(num) => Ty::Integer(IntegerKind::from_int_number(num)),
            _ => Ty::Unknown,
        }
    }

    fn infer_expected_call_arg_tys(&mut self, ty_callable: &TyCallable, expected: Expected) -> Vec<Ty> {
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
    }

    fn coerce_call_arg_types(
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
                    let arg_expr_ty = self.infer_expr(&expr, Expected::ExpectType(expected_ty.clone()));
                    self.ctx
                        .coerce_types(expr.node_or_token(), arg_expr_ty, expected_ty);
                }
            }
        }
    }
}

enum CallArg {
    Self_ { self_ty: Ty },
    Arg { expr: ast::Expr },
}
