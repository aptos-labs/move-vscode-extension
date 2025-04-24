mod infer_specs;

use crate::nameres::name_resolution::get_entries_from_walking_scopes;
use crate::nameres::namespaces::NAMES;
use crate::nameres::path_resolution::get_method_resolve_variants;
use crate::nameres::scope::{ScopeEntryExt, ScopeEntryListExt, VecExt};
use crate::node_ext::item_spec::ItemSpecExt;
use crate::types::expectation::Expected;
use crate::types::inference::InferenceCtx;
use crate::types::patterns::{BindingMode, anonymous_pat_ty_var};
use crate::types::substitution::ApplySubstitution;
use crate::types::ty::Ty;
use crate::types::ty::integer::IntegerKind;
use crate::types::ty::range_like::TySequence;
use crate::types::ty::reference::{Mutability, autoborrow};
use crate::types::ty::ty_callable::{CallKind, TyCallable};
use crate::types::ty::ty_var::{TyInfer, TyIntVar};
use std::iter;
use std::ops::Deref;
use syntax::ast::node_ext::named_field::FilterNamedFieldsByName;
use syntax::ast::{FieldsOwner, HasStmts};
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, IntoNodeOrToken, ast};

pub struct TypeAstWalker<'a, 'db> {
    pub ctx: &'a mut InferenceCtx<'db>,
    pub expected_return_ty: Ty,
}

impl<'a, 'db> TypeAstWalker<'a, 'db> {
    pub fn new(ctx: &'a mut InferenceCtx<'db>, expected_return_ty: Ty) -> Self {
        TypeAstWalker {
            ctx,
            expected_return_ty,
        }
    }

    pub fn walk(&mut self, ctx_owner: ast::InferenceCtxOwner) {
        self.collect_parameter_bindings(&ctx_owner);

        match ctx_owner {
            ast::InferenceCtxOwner::Fun(fun) => {
                if let Some(fun_block_expr) = fun.body() {
                    self.infer_block_expr(&fun_block_expr, Expected::NoValue);
                }
            }
            ast::InferenceCtxOwner::SpecFun(spec_fun) => {
                if let Some(spec_block_expr) = spec_fun.spec_block() {
                    self.process_msl_block_expr(&spec_block_expr);
                }
            }
            ast::InferenceCtxOwner::SpecInlineFun(spec_fun) => {
                if let Some(spec_block_expr) = spec_fun.spec_block() {
                    self.process_msl_block_expr(&spec_block_expr);
                }
            }
            ast::InferenceCtxOwner::ItemSpec(item_spec) => {
                if let Some(block_expr) = item_spec.spec_block() {
                    self.process_msl_block_expr(&block_expr);
                }
            }
            ast::InferenceCtxOwner::Schema(schema) => {
                if let Some(block_expr) = schema.spec_block() {
                    self.process_msl_block_expr(&block_expr);
                }
            }
        }

        self.walk_lambda_expr_bodies();
    }

    pub fn walk_lambda_expr_bodies(&mut self) {
        //  1. collect lambda expr bodies while inferring the context
        //  2. for every lambda expr body:
        //     1. infer lambda expr body, adding items to outer inference result
        //     2. resolve all vars again in the InferenceContext
        //  3. resolve vars replacing unresolved vars with Ty::Unknown
        while !self.ctx.lambda_exprs.is_empty() {
            self.ctx.resolve_all_ty_vars_if_possible();
            let lambda_expr = self.ctx.lambda_exprs.remove(0);
            let lambda_ret_ty = self
                .ctx
                .lambda_expr_types
                .get(&lambda_expr)
                .map(|it| it.ret_type());
            if let Some(body_expr) = lambda_expr.body_expr() {
                // todo: add coerce here
                self.infer_expr(&body_expr, Expected::from_ty(lambda_ret_ty));
            }
        }
    }

    pub fn collect_parameter_bindings(&mut self, ctx_owner: &ast::InferenceCtxOwner) -> Option<()> {
        let mut binding_file_id = self.ctx.file_id;
        let bindings = match ctx_owner {
            ast::InferenceCtxOwner::Fun(fun) => fun.to_any_fun().params_as_bindings(),
            ast::InferenceCtxOwner::SpecFun(fun) => fun.to_any_fun().params_as_bindings(),
            ast::InferenceCtxOwner::SpecInlineFun(fun) => fun.to_any_fun().params_as_bindings(),
            ast::InferenceCtxOwner::ItemSpec(item_spec) => {
                let item = item_spec.clone().in_file(self.ctx.file_id).item(self.ctx.db)?;
                binding_file_id = item.file_id;

                let item = item.value;
                match item {
                    ast::Item::Fun(fun) => fun.to_any_fun().params_as_bindings(),
                    ast::Item::SpecFun(fun) => fun.to_any_fun().params_as_bindings(),
                    _ => vec![],
                }
            }
            _ => {
                return None;
            }
        };
        for binding in bindings {
            let binding_ty = {
                let binding_type_owner = binding.owner();
                let ty_lowering = self.ctx.ty_lowering();
                match binding_type_owner {
                    Some(ast::IdentPatKind::Param(fun_param)) => fun_param
                        .type_()
                        .map(|it| ty_lowering.lower_type(it.in_file(binding_file_id)))
                        .unwrap_or(Ty::Unknown),
                    _ => continue,
                }
            };
            self.ctx.pat_types.insert(binding.into(), binding_ty);
        }
        Some(())
    }

    pub fn process_msl_block_expr(&mut self, block_expr: &ast::BlockExpr) -> Option<()> {
        self.ctx.msl_scope(|ctx| {
            let mut w = TypeAstWalker::new(ctx, Ty::Unit);
            w.infer_block_expr(&block_expr, Expected::NoValue);
        });
        Some(())
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

    fn process_stmt(&mut self, stmt: ast::Stmt) -> Option<()> {
        let file_id = self.ctx.file_id;
        match stmt {
            ast::Stmt::LetStmt(let_stmt) => {
                let explicit_ty = let_stmt
                    .type_()
                    .map(|it| self.ctx.ty_lowering().lower_type(it.in_file(file_id)));
                let initializer_ty = match let_stmt.initializer() {
                    Some(initializer_expr) => {
                        let initializer_ty =
                            self.infer_expr(&initializer_expr, Expected::from_ty(explicit_ty.clone()));
                        if let Some(explicit_ty) = explicit_ty.clone() {
                            self.ctx.coerce_types(
                                initializer_expr.node_or_token(),
                                initializer_ty.clone(),
                                explicit_ty.clone(),
                            );
                            // return explicit because it can be more specific
                            explicit_ty
                        } else {
                            initializer_ty
                        }
                    }
                    None => let_stmt
                        .pat()
                        .clone()
                        .map(|it| anonymous_pat_ty_var(self.ctx, &it))
                        .unwrap_or(Ty::Unknown),
                };
                let pat = let_stmt.pat()?;
                let pat_ty = explicit_ty.unwrap_or(self.ctx.resolve_ty_vars_if_possible(initializer_ty));
                self.collect_pat_bindings(pat, pat_ty, BindingMode::BindByValue);
            }
            ast::Stmt::ExprStmt(expr_stmt) => {
                let expr = expr_stmt.expr()?;
                self.infer_expr(&expr, Expected::NoValue);
            }
            ast::Stmt::SpecPredicateStmt(spec_predicate_stmt) => {
                self.process_predicate_stmt(&spec_predicate_stmt);
            }
            ast::Stmt::AbortsIfStmt(aborts_if_stmt) => {
                self.process_aborts_if_stmt(&aborts_if_stmt);
            }
            ast::Stmt::SchemaFieldStmt(schema_field) => {
                let ident_pat = schema_field.ident_pat()?;
                let ident_pat_ty = schema_field
                    .type_()
                    .map(|t| self.ctx.ty_lowering().lower_type(t.in_file(file_id)))
                    .unwrap_or(Ty::Unknown);
                self.ctx.pat_types.insert(
                    ident_pat.into(),
                    self.ctx.resolve_ty_vars_if_possible(ident_pat_ty),
                );
            }
            _ => (),
        }

        Some(())
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
        if no_type_error { expected_ty } else { actual_ty }
    }

    fn infer_expr(&mut self, expr: &ast::Expr, expected: Expected) -> Ty {
        if self.ctx.expr_types.contains_key(expr) {
            unreachable!("trying to infer expr twice");
        }

        let expected_ty = expected.ty(self.ctx);
        if let Some(expected_ty) = expected_ty {
            use syntax::SyntaxKind::*;

            if matches!(
                expr.syntax().kind(),
                STRUCT_LIT | PATH_EXPR | DOT_EXPR | METHOD_CALL_EXPR | CALL_EXPR
            ) {
                self.ctx.expected_expr_types.insert(expr.to_owned(), expected_ty);
            }
        }

        let expr_ty = match expr {
            ast::Expr::PathExpr(path_expr) => {
                self.infer_path_expr(path_expr, expected).unwrap_or(Ty::Unknown)
            }

            ast::Expr::CallExpr(call_expr) => self
                .infer_call_expr(call_expr, Expected::NoValue)
                .unwrap_or(Ty::Unknown),

            ast::Expr::MethodCallExpr(method_call_expr) => {
                self.infer_method_call_expr(method_call_expr, Expected::NoValue)
            }
            ast::Expr::VectorLitExpr(vector_lit_expr) => {
                self.infer_vector_lit_expr(vector_lit_expr, expected)
            }
            ast::Expr::TupleExpr(tuple_expr) => self.infer_tuple_expr(tuple_expr, expected),
            ast::Expr::RangeExpr(range_expr) => self.infer_range_expr(range_expr, expected),
            ast::Expr::StructLit(struct_lit) => {
                self.infer_struct_lit(struct_lit, expected).unwrap_or(Ty::Unknown)
            }

            ast::Expr::DotExpr(dot_expr) => self
                .infer_dot_expr(dot_expr, Expected::NoValue)
                .unwrap_or(Ty::Unknown),

            ast::Expr::AssertMacroExpr(assert_macro_expr) => {
                self.infer_assert_macro_expr(assert_macro_expr)
            }

            ast::Expr::IfExpr(if_expr) => self.infer_if_expr(if_expr, expected).unwrap_or(Ty::Unknown),
            ast::Expr::LoopExpr(loop_expr) => self.infer_loop_expr(loop_expr),
            ast::Expr::WhileExpr(while_expr) => self.infer_while_expr(while_expr),
            ast::Expr::ForExpr(for_expr) => self.infer_for_expr(for_expr),

            ast::Expr::BreakExpr(_) => Ty::Never,
            ast::Expr::ContinueExpr(_) => Ty::Never,
            ast::Expr::ReturnExpr(return_expr) => {
                if let Some(expr) = return_expr.expr() {
                    let expected = Expected::ExpectType(self.expected_return_ty.clone());
                    self.infer_expr(&expr, expected);
                }
                Ty::Never
            }
            ast::Expr::CastExpr(cast_expr) => self.infer_cast_expr(cast_expr, expected),

            ast::Expr::LambdaExpr(lambda_expr) => self.infer_lambda_expr(lambda_expr, expected),
            ast::Expr::MatchExpr(match_expr) => self.infer_match_expr(match_expr, expected),

            ast::Expr::ParenExpr(paren_expr) => paren_expr
                .expr()
                .map(|it| self.infer_expr(&it, expected))
                .unwrap_or(Ty::Unknown),

            ast::Expr::BorrowExpr(borrow_expr) => self
                .infer_borrow_expr(borrow_expr, expected)
                .unwrap_or(Ty::Unknown),

            ast::Expr::DerefExpr(deref_expr) => {
                self.infer_deref_expr(deref_expr, expected).unwrap_or(Ty::Unknown)
            }
            ast::Expr::IndexExpr(index_expr) => self.infer_index_expr(index_expr),

            ast::Expr::ResourceExpr(res_expr) => {
                self.infer_resource_expr(res_expr).unwrap_or(Ty::Unknown)
            }
            ast::Expr::IsExpr(is_expr) => self.infer_is_expr(is_expr),
            ast::Expr::AbortExpr(abort_expr) => {
                if let Some(inner_expr) = abort_expr.expr() {
                    self.infer_expr(&inner_expr, Expected::NoValue);
                }
                Ty::Never
            }

            ast::Expr::BlockExpr(block_expr) => self.infer_block_expr(block_expr, expected),
            ast::Expr::BinExpr(bin_expr) => self.infer_bin_expr(bin_expr).unwrap_or(Ty::Unknown),

            ast::Expr::BangExpr(bang_expr) => bang_expr
                .expr()
                .map(|it| {
                    self.infer_expr(&it, Expected::ExpectType(Ty::Bool));
                    Ty::Bool
                })
                .unwrap_or(Ty::Unknown),

            ast::Expr::Literal(lit) => self.infer_literal(lit),

            ast::Expr::ForallExpr(it) => self.infer_quant_expr(&it.clone().into()).unwrap_or(Ty::Bool),
            ast::Expr::ExistsExpr(it) => self.infer_quant_expr(&it.clone().into()).unwrap_or(Ty::Bool),
            ast::Expr::ChooseExpr(it) => self.infer_quant_expr(&it.clone().into()).unwrap_or(Ty::Bool),

            ast::Expr::SpecBlockExpr(it) => {
                if let Some(block_expr) = it.block_expr() {
                    self.process_msl_block_expr(&block_expr);
                }
                Ty::Unit
            }
        };

        let expr_ty = expr_ty.refine_for_specs(self.ctx.msl);
        self.ctx.expr_types.insert(expr.to_owned(), expr_ty.clone());

        expr_ty
    }

    fn infer_path_expr(&mut self, path_expr: &ast::PathExpr, expected: Expected) -> Option<Ty> {
        use syntax::SyntaxKind::*;

        let expected_ty = expected.ty(self.ctx);
        let named_element = self.ctx.resolve_path_cached(path_expr.path(), expected_ty)?;

        let file_id = self.ctx.file_id;
        let ty_lowering = self.ctx.ty_lowering();
        match named_element.kind() {
            IDENT_PAT => {
                let ident_pat = named_element.cast_into::<ast::IdentPat>()?.value;
                self.ctx.get_binding_type(ident_pat)
            }
            CONST => {
                let const_type = named_element
                    .cast_into::<ast::Const>()?
                    .and_then(|it| it.type_())?;
                Some(ty_lowering.lower_type(const_type))
            }
            NAMED_FIELD => {
                let named_field = named_element.cast_into::<ast::NamedField>()?;
                ty_lowering.lower_named_field(named_field)
            }
            STRUCT | ENUM => {
                // base for index expr
                let path = path_expr.path().in_file(file_id);
                let index_base_ty = ty_lowering.lower_path(path.map_into(), named_element.map_into());
                Some(index_base_ty)
            }
            VARIANT => {
                // MyEnum::MyVariant
                let variant = named_element.cast_into::<ast::Variant>().unwrap();
                let enum_path = path_expr.path().qualifier().unwrap_or(path_expr.path());
                let variant_ty = self
                    .ctx
                    .instantiate_path(enum_path.into(), variant.map(|it| it.enum_()).map_into());
                let variant_ty_adt = variant_ty.into_ty_adt()?;
                Some(Ty::Adt(variant_ty_adt))
            }
            MODULE => None,
            FUN | SPEC_FUN | SPEC_INLINE_FUN => {
                let any_fun = named_element.cast_into::<ast::AnyFun>().unwrap();
                let method_or_path: ast::MethodOrPath = path_expr.path().into();
                Some(self.ctx.instantiate_path_for_fun(method_or_path, any_fun).into())
            }
            _ => None,
        }
    }

    fn infer_dot_expr(&mut self, dot_expr: &ast::DotExpr, _expected: Expected) -> Option<Ty> {
        let self_ty = self.infer_expr(&dot_expr.receiver_expr(), Expected::NoValue);
        let self_ty = self.ctx.resolve_ty_vars_if_possible(self_ty);

        let ty_adt = self_ty.unwrap_all_refs().into_ty_adt()?;

        let field_ref = dot_expr.field_ref();
        if !self.ctx.msl && field_ref.containing_module() != ty_adt.adt_item_module(self.ctx.db) {
            return None;
        }

        let adt_item = ty_adt
            .adt_item_loc
            .to_ast::<ast::StructOrEnum>(self.ctx.db.upcast())?;
        let field_reference_name = dot_expr.field_ref().name_ref()?.as_string();

        // todo: tuple index fields

        let InFile {
            file_id: adt_item_file_id,
            value: adt_item,
        } = adt_item;
        let named_field = adt_item
            .field_ref_lookup_fields()
            .filter_fields_by_name(&field_reference_name)
            .single_or_none()
            .map(|it| it.in_file(adt_item_file_id));

        self.ctx.resolved_fields.insert(
            dot_expr.field_ref(),
            named_field.clone().and_then(|it| it.to_entry()),
        );

        let ty_lowering = self.ctx.ty_lowering();
        let named_field_type = named_field?.and_then(|it| it.type_())?;
        let field_ty = ty_lowering
            .lower_type(named_field_type)
            .substitute(&ty_adt.substitution);
        Some(field_ty)
    }

    fn infer_method_call_expr(
        &mut self,
        method_call_expr: &ast::MethodCallExpr,
        expected: Expected,
    ) -> Ty {
        let self_ty = self.infer_expr(&method_call_expr.receiver_expr(), Expected::NoValue);
        let self_ty = self.ctx.resolve_ty_vars_if_possible(self_ty);

        let method_entry =
            get_method_resolve_variants(self.ctx.db, &self_ty, self.ctx.file_id, self.ctx.msl)
                .filter_by_name(method_call_expr.reference_name())
                .filter_by_visibility(self.ctx.db, &method_call_expr.clone().in_file(self.ctx.file_id))
                .single_or_none();
        self.ctx
            .resolved_method_calls
            .insert(method_call_expr.to_owned(), method_entry.clone());

        let resolved_method =
            method_entry.and_then(|it| it.node_loc.to_ast::<ast::Fun>(self.ctx.db.upcast()));
        let method_ty = match resolved_method {
            Some(method) => self
                .ctx
                .instantiate_path_for_fun(method_call_expr.to_owned().into(), method.map_into()),
            None => {
                // add 1 for `self` parameter
                TyCallable::fake(1 + method_call_expr.args().len(), CallKind::Fun)
            }
        };
        let method_ty = self.ctx.resolve_ty_vars_if_possible(method_ty);

        let expected_arg_tys = self.infer_expected_call_arg_tys(&method_ty, expected);
        let args = iter::once(CallArg::Self_ { self_ty })
            .chain(
                method_call_expr
                    .args()
                    .into_iter()
                    .map(|arg_expr| CallArg::Arg { expr: arg_expr }),
            )
            .collect();
        self.coerce_call_arg_types(args, method_ty.param_types.clone(), expected_arg_tys);

        method_ty.ret_type()
    }

    fn infer_call_expr(&mut self, call_expr: &ast::CallExpr, expected: Expected) -> Option<Ty> {
        let lhs_expr = call_expr.expr()?;
        let lhs_ty = self.infer_expr(&lhs_expr, Expected::NoValue);
        let callable_ty = match lhs_ty {
            Ty::Callable(ty_callable) => ty_callable,
            Ty::Adt(ty_adt) => {
                let path = call_expr.path()?;
                let callable_ty = self.ctx.instantiate_adt_item_as_callable(path, ty_adt)?;
                callable_ty
            }
            _ => {
                return None;
            }
        };
        let expected_arg_tys = self.infer_expected_call_arg_tys(&callable_ty, expected);
        let args = call_expr
            .args()
            .into_iter()
            .map(|expr| CallArg::Arg { expr })
            .collect();
        self.coerce_call_arg_types(args, callable_ty.param_types.clone(), expected_arg_tys);

        Some(callable_ty.ret_type())
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

    fn infer_struct_lit(&mut self, struct_lit: &ast::StructLit, expected: Expected) -> Option<Ty> {
        let path = struct_lit.path();
        let expected_ty = expected.ty(self.ctx);
        let item = self.ctx.resolve_path_cached(path.clone(), expected_ty.clone())?;
        let (item_file_id, item) = item.unpack();

        let fields_owner = item.cast_into::<ast::AnyFieldsOwner>();
        if fields_owner.is_none() {
            for field in struct_lit.fields() {
                if let Some(field_expr) = field.expr() {
                    self.infer_expr(&field_expr, Expected::NoValue);
                }
            }
            return None;
        }
        let fields_owner = fields_owner.unwrap();

        let struct_or_enum = fields_owner.struct_or_enum();
        let mut ty_adt = self
            .ctx
            .instantiate_path(path.into(), struct_or_enum.in_file(item_file_id).map_into())
            .into_ty_adt()?;
        if let Some(Ty::Adt(expected_ty_adt)) = expected_ty {
            let expected_subst = expected_ty_adt.substitution;
            for (type_param, subst_ty) in ty_adt.substitution.entries() {
                // skip type parameters as we have no ability check
                if matches!(subst_ty, &Ty::TypeParam(_)) {
                    continue;
                }
                if let Some(expected_subst_ty) = expected_subst.get_ty(&type_param) {
                    // unifies if `substTy` is TyVar, performs type check if `substTy` is real type
                    let _ = self.ctx.combine_types(subst_ty.to_owned(), expected_subst_ty);
                }
            }
            // resolved tyAdt inner TyVars after combining with expectedTy
            ty_adt = self.ctx.resolve_ty_vars_if_possible(ty_adt)
        }

        let named_fields = fields_owner.named_fields_map();
        for lit_field in struct_lit.fields() {
            let lit_field_name = lit_field.field_name();
            if lit_field_name.is_none() {
                continue;
            }
            let lit_field_name = lit_field_name.unwrap();

            let named_field = named_fields.get(&lit_field_name);
            let declared_field_ty = named_field
                .and_then(|it| {
                    self.ctx
                        .ty_lowering()
                        .lower_named_field(it.to_owned().in_file(item_file_id))
                })
                .unwrap_or(Ty::Unknown);
            let field_ty = declared_field_ty.substitute(&ty_adt.substitution);

            if let Some(lit_field_expr) = lit_field.expr() {
                self.infer_expr_coerceable_to(&lit_field_expr, field_ty);
            } else {
                let binding = get_entries_from_walking_scopes(
                    self.ctx.db,
                    lit_field.clone().in_file(self.ctx.file_id),
                    NAMES,
                )
                .filter_by_name(lit_field_name)
                .single_or_none()
                .and_then(|it| it.cast_into::<ast::IdentPat>(self.ctx.db));
                let binding_ty = binding
                    .and_then(|it| self.ctx.get_binding_type(it.value))
                    .unwrap_or(Ty::Unknown);
                self.ctx
                    .coerce_types(lit_field.node_or_token(), binding_ty, field_ty);
            }
        }

        Some(Ty::Adt(ty_adt))
    }

    fn infer_index_expr(&mut self, index_expr: &ast::IndexExpr) -> Ty {
        let Some(arg_expr) = index_expr.arg_expr() else {
            return Ty::Unknown;
        };
        let base_ty = self.infer_expr(&index_expr.base_expr(), Expected::NoValue);
        let deref_ty = self
            .ctx
            .resolve_ty_vars_if_possible(base_ty.clone())
            .unwrap_all_refs();
        let arg_ty = self.infer_expr(&arg_expr, Expected::NoValue);

        if let Ty::Seq(TySequence::Vector(item_ty)) = deref_ty.clone() {
            let item_ty = item_ty.deref().to_owned();
            // arg_ty can be either TyInteger or TyRange
            return match arg_ty {
                Ty::Seq(TySequence::Range(_)) => deref_ty,
                Ty::Integer(_) | Ty::Infer(TyInfer::IntVar(_)) | Ty::Num => item_ty,
                _ => {
                    self.ctx.coerce_types(
                        arg_expr.node_or_token(),
                        item_ty.clone(),
                        if self.ctx.msl {
                            Ty::Num
                        } else {
                            Ty::Integer(IntegerKind::Integer)
                        },
                    );
                    item_ty
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

    fn infer_lambda_expr(&mut self, lambda_expr: &ast::LambdaExpr, expected: Expected) -> Ty {
        let mut param_tys = vec![];

        for (lambda_param, ident_pat) in lambda_expr.params_with_ident_pats() {
            let file_id = self.ctx.file_id;
            let param_ty = match lambda_param.type_() {
                Some(type_) => self.ctx.ty_lowering().lower_type(type_.in_file(file_id)),
                None => Ty::new_ty_var(self.ctx),
            };
            self.ctx.pat_types.insert(ident_pat.into(), param_ty.clone());
            param_tys.push(param_ty);
        }

        let lambda_call_ty = TyCallable::new(param_tys, Ty::new_ty_var(self.ctx), CallKind::Lambda);
        self.ctx.lambda_exprs.push(lambda_expr.clone());
        self.ctx
            .lambda_expr_types
            .insert(lambda_expr.clone(), lambda_call_ty.clone());

        let lambda_ty = Ty::Callable(lambda_call_ty);
        if let Some(expected_ty) = expected.ty(self.ctx) {
            // error if not TyCallable
            self.ctx
                .coerce_types(lambda_expr.node_or_token(), lambda_ty.clone(), expected_ty);
        }

        lambda_ty
    }

    fn infer_match_expr(&mut self, match_expr: &ast::MatchExpr, _expected: Expected) -> Ty {
        let match_arg_ty = match_expr
            .expr()
            .map(|expr| {
                let expr_ty = self.infer_expr(&expr, Expected::NoValue);
                self.ctx.resolve_ty_vars_if_possible(expr_ty)
            })
            .unwrap_or(Ty::Unknown);

        let arms = match_expr.arms();
        let mut arm_tys = vec![];
        for arm in arms {
            if let Some(pat) = arm.pat() {
                self.collect_pat_bindings(pat, match_arg_ty.clone(), BindingMode::BindByValue);
            }
            if let Some(match_guard_expr) = arm.match_guard().and_then(|it| it.expr()) {
                self.infer_expr(&match_guard_expr, Expected::ExpectType(Ty::Bool));
            }
            if let Some(arm_expr) = arm.expr() {
                let arm_ty = self.infer_expr(&arm_expr, Expected::NoValue);
                arm_tys.push(arm_ty);
            }
        }

        self.ctx.intersect_all_types(arm_tys)
    }

    fn infer_range_expr(&mut self, range_expr: &ast::RangeExpr, _expected: Expected) -> Ty {
        let start_ty = self.infer_expr(&range_expr.start_expr(), Expected::NoValue);
        if let Some(end_expr) = range_expr.end_expr() {
            self.infer_expr_coerceable_to(&end_expr, start_ty.clone());
        }
        Ty::Seq(TySequence::Range(Box::new(start_ty)))
    }

    fn infer_vector_lit_expr(&mut self, vector_lit_expr: &ast::VectorLitExpr, expected: Expected) -> Ty {
        let arg_ty_var = Ty::new_ty_var(self.ctx);

        let explicit_ty = vector_lit_expr.type_arg().map(|it| {
            let file_id = self.ctx.file_id;
            self.ctx.ty_lowering().lower_type(it.type_().in_file(file_id))
        });
        if let Some(explicit_ty) = explicit_ty {
            let _ = self.ctx.combine_types(arg_ty_var.clone(), explicit_ty);
        }

        let arg_ty = self.ctx.resolve_ty_vars_if_possible(arg_ty_var.clone());
        let arg_exprs = vector_lit_expr.arg_exprs().collect::<Vec<_>>();
        let declared_arg_tys = iter::repeat_n(arg_ty, arg_exprs.len()).collect::<Vec<_>>();

        let vec_ty = Ty::new_vector(arg_ty_var);

        let lit_call_ty = TyCallable::new(declared_arg_tys.clone(), vec_ty.clone(), CallKind::Fun);
        let expected_arg_tys = self.infer_expected_call_arg_tys(&lit_call_ty, expected);
        let args = arg_exprs.into_iter().map(|expr| CallArg::Arg { expr }).collect();
        self.coerce_call_arg_types(args, declared_arg_tys, expected_arg_tys);

        self.ctx.resolve_ty_vars_if_possible(vec_ty)
    }

    fn infer_tuple_expr(&mut self, tuple_expr: &ast::TupleExpr, expected: Expected) -> Ty {
        let expected_tys = expected
            .ty(self.ctx)
            .and_then(|it| it.into_ty_tuple())
            .map(|it| it.types)
            .unwrap_or_default();

        let mut tys = vec![];
        for (i, expr) in tuple_expr.exprs().enumerate() {
            let expr_ty = expected_tys.get(i).cloned();
            tys.push(
                match expr_ty {
                    Some(ty) => self.infer_expr_coerceable_to(&expr, ty),
                    None => self.infer_expr(&expr, Expected::NoValue),
                }
                .into(),
            );
        }

        Ty::new_tuple(tys)
    }

    fn infer_if_expr(&mut self, if_expr: &ast::IfExpr, expected: Expected) -> Option<Ty> {
        let condition_expr = if_expr.condition()?.expr()?;
        self.infer_expr_coerceable_to(&condition_expr, Ty::Bool);

        let actual_if_ty = if_expr
            .then_branch()
            .map(|it| self.infer_block_or_inline_expr(&it, expected.clone()));
        let Some(else_branch) = if_expr.else_branch() else {
            return Some(Ty::Unit);
        };

        let expected_else_ty = expected
            .ty(self.ctx)
            .or(actual_if_ty.clone())
            .unwrap_or(Ty::Unknown);
        let actual_else_ty = self.infer_block_or_inline_expr(&else_branch, expected);

        if let Some(tail_expr) = else_branch.tail_expr() {
            // `if (true) &s else &mut s` shouldn't show type error
            self.ctx.coerce_types(
                tail_expr.node_or_token(),
                actual_else_ty.clone(),
                expected_else_ty,
            );
        }

        let tys = vec![actual_if_ty, Some(actual_else_ty)]
            .into_iter()
            .filter_map(|it| it)
            .collect();
        Some(self.ctx.intersect_all_types(tys))
    }

    fn infer_while_expr(&mut self, while_expr: &ast::WhileExpr) -> Ty {
        let condition_expr = while_expr.condition().and_then(|it| it.expr());
        if let Some(condition_expr) = condition_expr {
            self.infer_expr_coerceable_to(&condition_expr, Ty::Bool);
        }
        self.infer_loop_like_body(while_expr)
    }

    fn infer_for_expr(&mut self, for_expr: &ast::ForExpr) -> Ty {
        if let Some(for_condition) = for_expr.for_condition() {
            let seq_ty = for_condition.expr().and_then(|range_expr| {
                let range_ty = self.infer_expr(&range_expr, Expected::NoValue);
                self.ctx.resolve_ty_vars_if_possible(range_ty).into_ty_seq()
            });
            if let Some(ident_pat) = for_condition.ident_pat() {
                self.ctx.pat_types.insert(
                    ident_pat.into(),
                    seq_ty.map(|it| it.item()).unwrap_or(Ty::Unknown),
                );
            }
        }
        self.infer_loop_like_body(for_expr)
    }

    fn infer_loop_expr(&mut self, loop_expr: &ast::LoopExpr) -> Ty {
        self.infer_loop_like_body(loop_expr)
    }

    fn infer_loop_like_body(&mut self, loop_like: &impl ast::LoopLike) -> Ty {
        if let Some(loop_body_expr) = loop_like.loop_body_expr() {
            self.infer_block_or_inline_expr(&loop_body_expr, Expected::ExpectType(Ty::Unit));
        }
        Ty::Never
    }

    fn infer_cast_expr(&mut self, cast_expr: &ast::CastExpr, expected: Expected) -> Ty {
        self.infer_expr(&cast_expr.expr(), Expected::NoValue);
        if let Some(type_) = cast_expr.type_() {
            let file_id = self.ctx.file_id;
            let ty = self.ctx.ty_lowering().lower_type(type_.in_file(file_id));
            if let Some(expected_ty) = expected.ty(self.ctx) {
                let _ = self.ctx.combine_types(expected_ty, ty.clone());
            }
            return ty;
        }
        Ty::Unknown
    }

    fn infer_block_or_inline_expr(
        &mut self,
        block_or_inline_expr: &ast::BlockOrInlineExpr,
        expected: Expected,
    ) -> Ty {
        match block_or_inline_expr {
            ast::BlockOrInlineExpr::BlockExpr(block_expr) => self.infer_block_expr(block_expr, expected),
            ast::BlockOrInlineExpr::InlineExpr(inline_expr) => inline_expr
                .expr()
                .map(|expr| self.infer_expr(&expr, expected))
                .unwrap_or(Ty::Unknown),
        }
    }

    fn infer_borrow_expr(&mut self, borrow_expr: &ast::BorrowExpr, expected: Expected) -> Option<Ty> {
        let inner_expr = borrow_expr.expr()?;
        let inner_expected_ty = expected
            .ty(self.ctx)
            .and_then(|ty| ty.into_ty_ref())
            .map(|ty_ref| ty_ref.referenced());

        let inner_ty = self.infer_expr(&inner_expr, Expected::from_ty(inner_expected_ty));
        let mutability = Mutability::new(borrow_expr.is_mut());

        Some(Ty::new_reference(inner_ty, mutability))
    }

    fn infer_deref_expr(&mut self, deref_expr: &ast::DerefExpr, _expected: Expected) -> Option<Ty> {
        let inner_expr = deref_expr.expr()?;
        let inner_ty = self.infer_expr(&inner_expr, Expected::NoValue);

        // todo: error
        let inner_ty_ref = inner_ty.into_ty_ref()?;

        Some(inner_ty_ref.referenced())
    }

    fn infer_resource_expr(&mut self, resource_expr: &ast::ResourceExpr) -> Option<Ty> {
        let inner_expr = resource_expr.expr()?;
        let inner_ty = self.infer_expr(&inner_expr, Expected::NoValue);
        Some(inner_ty)
    }

    fn infer_is_expr(&mut self, is_expr: &ast::IsExpr) -> Ty {
        let expr_ty = self.infer_expr(&is_expr.expr(), Expected::NoValue);
        for path_type in is_expr.path_types() {
            self.ctx
                .resolve_path_cached(path_type.path(), Some(expr_ty.clone()));
        }
        Ty::Bool
    }

    fn infer_bin_expr(&mut self, bin_expr: &ast::BinExpr) -> Option<Ty> {
        let (lhs, (_, op_kind), rhs) = bin_expr.unpack()?;
        let ty = match op_kind {
            ast::BinaryOp::ArithOp(_) => self.infer_arith_binary_expr(lhs, rhs, false),

            ast::BinaryOp::Assignment { op: None } => self.infer_assignment(lhs, rhs),
            ast::BinaryOp::Assignment { op: Some(_) } => self.infer_arith_binary_expr(lhs, rhs, true),

            ast::BinaryOp::LogicOp(_) => self.infer_logic_binary_expr(lhs, rhs),

            ast::BinaryOp::CmpOp(op) => match op {
                ast::CmpOp::Eq { .. } => self.infer_eq_binary_expr(&lhs, &rhs),
                ast::CmpOp::Ord { .. } => self.infer_ordering_binary_expr(&lhs, &rhs),
            },
        };
        Some(ty)
    }

    fn infer_assignment(&mut self, lhs: ast::Expr, rhs: Option<ast::Expr>) -> Ty {
        let lhs_ty = self.infer_expr(&lhs, Expected::NoValue);
        if let Some(rhs) = rhs {
            self.infer_expr_coerceable_to(&rhs, lhs_ty);
        }
        Ty::Unit
    }

    fn infer_arith_binary_expr(
        &mut self,
        lhs: ast::Expr,
        rhs: Option<ast::Expr>,
        is_compound: bool,
    ) -> Ty {
        let mut is_error = false;
        let left_ty = self.infer_expr(&lhs, Expected::NoValue);
        if !left_ty.supports_arithm_op() {
            // todo: report error
            is_error = true;
        }
        if let Some(rhs) = rhs {
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
        }
        if is_error {
            Ty::Unknown
        } else {
            if is_compound { Ty::Unit } else { left_ty }
        }
    }

    fn infer_logic_binary_expr(&mut self, lhs: ast::Expr, rhs: Option<ast::Expr>) -> Ty {
        self.infer_expr_coerceable_to(&lhs, Ty::Bool);
        if let Some(rhs) = rhs {
            self.infer_expr_coerceable_to(&rhs, Ty::Bool);
        }
        Ty::Bool
    }

    fn infer_eq_binary_expr(&mut self, lhs: &ast::Expr, rhs: &Option<ast::Expr>) -> Ty {
        let left_ty = self.infer_expr(lhs, Expected::NoValue);
        let left_ty = self.ctx.resolve_ty_vars_if_possible(left_ty);

        if let Some(rhs) = rhs {
            let right_ty = self.infer_expr(rhs, Expected::NoValue);
            let right_ty = self.ctx.resolve_ty_vars_if_possible(right_ty);

            let combined = self.ctx.combine_types(left_ty, right_ty);
            if combined.is_err() {
                // todo: report error
            }
        }
        Ty::Bool
    }

    fn infer_ordering_binary_expr(&mut self, lhs: &ast::Expr, rhs: &Option<ast::Expr>) -> Ty {
        let mut is_error = false;
        let left_ty = self.infer_expr(lhs, Expected::NoValue);
        if !left_ty.supports_ordering() {
            // todo: report error
            is_error = true;
        }
        if let Some(rhs) = rhs {
            let right_ty = self.infer_expr(rhs, Expected::NoValue);
            if !right_ty.supports_ordering() {
                // todo: report error
                is_error = true;
            }
            if !is_error {
                self.ctx.coerce_types(rhs.node_or_token(), right_ty, left_ty);
            }
        }
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
            ast::LiteralKind::ByteString(_) | ast::LiteralKind::HexString(_) => {
                Ty::new_vector(Ty::Integer(IntegerKind::U8))
            }
            ast::LiteralKind::Invalid => Ty::Unknown,
        }
    }

    fn infer_expected_call_arg_tys(&mut self, ty_callable: &TyCallable, expected: Expected) -> Vec<Ty> {
        let Some(expected_ret_ty) = expected.ty(self.ctx) else {
            return vec![];
        };
        let declared_ret_ty = self.ctx.resolve_ty_vars_if_possible(ty_callable.ret_type());

        // unify return types and check if they are compatible
        let combined = self.ctx.combine_types(expected_ret_ty, declared_ret_ty);
        match combined {
            Ok(()) => ty_callable
                .param_types
                .iter()
                .map(|t| self.ctx.resolve_ty_vars_if_possible(t.clone()))
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
                .resolve_ty_vars_if_possible(expected_tys.get(i).unwrap_or(&declared_ty).to_owned());
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
