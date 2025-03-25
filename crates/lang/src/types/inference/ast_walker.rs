use crate::files::InFileExt;
use crate::nameres::path_resolution::get_method_resolve_variants;
use crate::nameres::scope::{ScopeEntryExt, ScopeEntryListExt, VecExt};
use crate::types::expectation::Expected;
use crate::types::fold::TypeFoldable;
use crate::types::inference::InferenceCtx;
use crate::types::patterns::{anonymous_pat_ty_var, collect_bindings, BindingMode};
use crate::types::substitution::ApplySubstitution;
use crate::types::ty::integer::IntegerKind;
use crate::types::ty::reference::{autoborrow, Mutability, TyReference};
use crate::types::ty::ty_callable::TyCallable;
use crate::types::ty::ty_var::{TyInfer, TyIntVar, TyVar};
use crate::types::ty::Ty;
use crate::InFile;
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
                        .map(|it| anonymous_pat_ty_var(self.ctx.inc_ty_counter(), &it))
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
                if let Some(expr) = expr_stmt.expr() {
                    self.infer_expr(&expr, Expected::NoValue);
                }
            }
            _ => {}
        }
    }

    // returns inferred
    fn infer_expr_coerceable_to(&mut self, expr: &ast::Expr, expected_ty: Ty) -> Ty {
        let actual_ty = self.infer_expr(expr, Expected::ExpectType(expected_ty.clone()));
        self.ctx
            .coerce_types(expr.node_or_token(), actual_ty.clone(), expected_ty);
        actual_ty
    }

    // returns expected
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
            ast::Expr::PathExpr(path_expr) => self
                .infer_path_expr(path_expr, Expected::NoValue)
                .unwrap_or(Ty::Unknown),

            ast::Expr::CallExpr(call_expr) => self.infer_call_expr(call_expr, Expected::NoValue),

            ast::Expr::MethodCallExpr(method_call_expr) => {
                self.infer_method_call_expr(method_call_expr, Expected::NoValue)
            }
            ast::Expr::VectorLitExpr(vector_lit_expr) => self.infer_vector_lit_expr(vector_lit_expr, expected),

            ast::Expr::DotExpr(dot_expr) => self
                .infer_dot_expr(dot_expr, Expected::NoValue)
                .unwrap_or(Ty::Unknown),

            ast::Expr::AssertMacroExpr(assert_macro_expr) => {
                self.infer_assert_macro_expr(assert_macro_expr)
            }

            ast::Expr::ParenExpr(paren_expr) => paren_expr
                .expr()
                .map(|it| self.infer_expr(&it, Expected::NoValue))
                .unwrap_or(Ty::Unknown),

            ast::Expr::BorrowExpr(borrow_expr) => self
                .infer_borrow_expr(borrow_expr, expected)
                .unwrap_or(Ty::Unknown),

            ast::Expr::DerefExpr(deref_expr) => self.infer_deref_expr(deref_expr).unwrap_or(Ty::Unknown),
            ast::Expr::IndexExpr(index_expr) => self.infer_index_expr(index_expr),

            ast::Expr::ResourceExpr(res_expr) => {
                self.infer_resource_expr(res_expr).unwrap_or(Ty::Unknown)
            }
            ast::Expr::AbortExpr(abort_expr) => {
                if let Some(inner_expr) = abort_expr.expr() {
                    self.infer_expr(&inner_expr, Expected::NoValue);
                }
                Ty::Never
            }

            ast::Expr::BlockExpr(block_expr) => self.infer_block_expr(block_expr, Expected::NoValue),
            ast::Expr::BinExpr(bin_expr) => self.infer_bin_expr(bin_expr).unwrap_or(Ty::Unknown),

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

    fn infer_path_expr(&mut self, path_expr: &ast::PathExpr, expected: Expected) -> Option<Ty> {
        use syntax::SyntaxKind::*;

        let expected_ty = expected.ty(self.ctx);
        let named_element = self.ctx.resolve_path_cached(path_expr.path(), expected_ty)?;

        let ty_lowering = self.ctx.ty_lowering();
        match named_element.kind() {
            IDENT_PAT => {
                let ident_pat = named_element.cast::<ast::IdentPat>()?.value;
                self.ctx.get_binding_type(ident_pat)
            }
            CONST => {
                let const_type = named_element.cast::<ast::Const>()?.value.type_()?;
                Some(ty_lowering.lower_type(const_type))
            }
            NAMED_FIELD => {
                let field_type = named_element.cast::<ast::NamedField>()?.value.type_()?;
                Some(ty_lowering.lower_type(field_type))
            }
            STRUCT | ENUM => {
                // base for index expr
                let index_base_ty = ty_lowering.lower_path(
                    path_expr.path().into(),
                    named_element.map(|it| it.syntax().to_owned()),
                );
                Some(index_base_ty)
            }
            MODULE => None,
            // todo: return TyCallable when "function values" feature is implemented
            FUN | SPEC_FUN | SPEC_INLINE_FUN => None,

            _ => None,
        }
    }

    fn infer_dot_expr(&mut self, dot_expr: &ast::DotExpr, _expected: Expected) -> Option<Ty> {
        let self_ty = self.infer_expr(&dot_expr.receiver_expr(), Expected::NoValue);
        let self_ty = self.ctx.resolve_vars_if_possible(self_ty);

        let ty_adt = self_ty.deref().into_ty_adt()?;
        let adt_item = ty_adt
            .adt_item
            .cast_into::<ast::StructOrEnum>(self.ctx.db.upcast())
            .unwrap();

        let field_reference_name = dot_expr.field_ref().name_ref()?.as_string();

        // todo: cannot resolve in outside of declared module
        // todo: tuple index fields

        let InFile {
            file_id: adt_item_file_id,
            value: adt_item,
        } = adt_item;
        let named_field = adt_item
            .field_ref_lookup_fields()
            .into_iter()
            .filter(|it| it.name().unwrap().as_string() == field_reference_name)
            .collect::<Vec<_>>()
            .single_or_none();

        self.ctx.resolved_fields.insert(
            dot_expr.field_ref(),
            named_field
                .clone()
                .and_then(|field| field.in_file(adt_item_file_id).to_entry()),
        );

        let ty_lowering = self.ctx.ty_lowering();
        let named_field_type = named_field.and_then(|it| it.type_())?;

        let field_ty = ty_lowering
            .lower_type(named_field_type)
            .substitute(ty_adt.substitution);
        Some(field_ty)
    }

    fn infer_method_call_expr(
        &mut self,
        method_call_expr: &ast::MethodCallExpr,
        expected: Expected,
    ) -> Ty {
        let self_ty = self.infer_expr(&method_call_expr.receiver_expr(), Expected::NoValue);
        let self_ty = self.ctx.resolve_vars_if_possible(self_ty);

        let method_entry = get_method_resolve_variants(self.ctx.db, &self_ty, self.ctx.file_id)
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

    fn infer_assert_macro_expr(&mut self, assert_macro_expr: &ast::AssertMacroExpr) -> Ty {
        let declared_input_tys = vec![Ty::Bool, Ty::Integer(IntegerKind::Integer)];
        let args = assert_macro_expr
            .args()
            .into_iter()
            .map(|expr| CallArg::Arg { expr })
            .collect();
        self.coerce_call_arg_types(args, declared_input_tys, vec![]);

        Ty::Unit
    }

    fn infer_index_expr(&mut self, index_expr: &ast::IndexExpr) -> Ty {
        let Some(arg_expr) = index_expr.arg_expr() else {
            return Ty::Unknown;
        };
        let base_ty = self.infer_expr(&index_expr.base_expr(), Expected::NoValue);
        let deref_ty = self.ctx.resolve_vars_if_possible(base_ty.clone()).deref();
        let arg_ty = self.infer_expr(&arg_expr, Expected::NoValue);

        if let Ty::Vector(item_ty) = deref_ty.clone() {
            // arg_ty can be either TyInteger or TyRange
            return match arg_ty {
                Ty::Range(_) => deref_ty,
                Ty::Integer(_) | Ty::Infer(TyInfer::IntVar(_)) | Ty::Num => item_ty.deref().to_owned(),
                _ => {
                    self.ctx.coerce_types(
                        arg_expr.node_or_token(),
                        arg_ty,
                        if self.ctx.msl {
                            Ty::Num
                        } else {
                            Ty::Integer(IntegerKind::Integer)
                        },
                    );
                    item_ty.deref().to_owned()
                }
            };
        }

        if let Ty::Adt(_) = base_ty.clone() {
            self.ctx
                .coerce_types(arg_expr.node_or_token(), arg_ty, Ty::Address);
            return base_ty;
        }

        Ty::Unknown
    }

    fn infer_vector_lit_expr(&mut self, vector_lit_expr: &ast::VectorLitExpr, expected: Expected) -> Ty {
        let arg_ty_var = Ty::Infer(TyInfer::Var(TyVar::new_anonymous(self.ctx.inc_ty_counter())));

        let explicit_ty = vector_lit_expr
            .type_arg()
            .map(|it| self.ctx.ty_lowering().lower_type(it.type_()));
        if let Some(explicit_ty) = explicit_ty {
            let _ = self.ctx.combine_types(arg_ty_var.clone(), explicit_ty);
        }

        let arg_ty = self.ctx.resolve_vars_if_possible(arg_ty_var.clone());
        let arg_exprs = vector_lit_expr.arg_exprs().collect::<Vec<_>>();
        let declared_arg_tys = iter::repeat_n(arg_ty, arg_exprs.len()).collect::<Vec<_>>();

        let vec_ty = Ty::Vector(Box::new(arg_ty_var));

        let lit_call_ty = TyCallable::new(declared_arg_tys.clone(), vec_ty.clone());
        let expected_arg_tys = self.infer_expected_call_arg_tys(&lit_call_ty, expected);
        let args = arg_exprs.into_iter().map(|expr| CallArg::Arg { expr }).collect();
        self.coerce_call_arg_types(args, declared_arg_tys, expected_arg_tys);

        self.ctx.resolve_vars_if_possible(vec_ty)
    }

    fn infer_borrow_expr(&mut self, borrow_expr: &ast::BorrowExpr, expected: Expected) -> Option<Ty> {
        let inner_expr = borrow_expr.expr()?;
        let inner_expected_ty = expected
            .ty(self.ctx)
            .and_then(|ty| ty.into_ty_ref())
            .map(|ty_ref| ty_ref.referenced.deref().to_owned());

        let inner_ty = self.infer_expr(&inner_expr, Expected::from_ty(inner_expected_ty));
        let mutability = Mutability::new(borrow_expr.is_mut());

        Some(Ty::Reference(TyReference::new(inner_ty, mutability)))
    }

    fn infer_deref_expr(&mut self, deref_expr: &ast::DerefExpr) -> Option<Ty> {
        let inner_expr = deref_expr.expr()?;
        let inner_ty = self.infer_expr(&inner_expr, Expected::NoValue);

        // todo: error
        let inner_ty_ref = inner_ty.into_ty_ref()?;

        Some(inner_ty_ref.referenced.deref().to_owned())
    }

    fn infer_resource_expr(&mut self, resource_expr: &ast::ResourceExpr) -> Option<Ty> {
        let inner_expr = resource_expr.expr()?;
        let inner_ty = self.infer_expr(&inner_expr, Expected::NoValue);
        Some(inner_ty)
    }

    fn infer_bin_expr(&mut self, bin_expr: &ast::BinExpr) -> Option<Ty> {
        let (lhs, (_, op_kind), rhs) = bin_expr.unpack()?;
        match op_kind {
            ast::BinaryOp::ArithOp(op) => Some(self.infer_arith_binary_expr(lhs, op, rhs, false)),
            ast::BinaryOp::LogicOp(op) => Some(self.infer_logic_binary_expr(lhs, op, rhs)),
            _ => None,
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

    fn infer_logic_binary_expr(&mut self, lhs: ast::Expr, _op: ast::LogicOp, rhs: ast::Expr) -> Ty {
        self.infer_expr_coerceable_to(&lhs, Ty::Bool);
        self.infer_expr_coerceable_to(&rhs, Ty::Bool);
        Ty::Bool
    }

    fn infer_literal(&mut self, literal: &ast::Literal) -> Ty {
        match literal.kind() {
            ast::LiteralKind::Bool(_) => Ty::Bool,
            ast::LiteralKind::IntNumber(num) => {
                let kind = IntegerKind::from_suffixed_literal(num);
                match kind {
                    Some(kind) => Ty::Integer(kind),
                    None => Ty::Infer(TyInfer::IntVar(TyIntVar::new(self.ctx.inc_ty_counter()))),
                }
            }
            ast::LiteralKind::Address(_) => Ty::Address,
            ast::LiteralKind::ByteString(_) => Ty::Vector(Box::new(Ty::Integer(IntegerKind::U8))),
            ast::LiteralKind::Invalid => Ty::Unknown,
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
