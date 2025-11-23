// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

mod infer_specs;
mod lambda_expr;

use crate::nameres::is_visible::is_visible_in_context;
use crate::nameres::name_resolution::{WalkScopesCtx, get_entries_from_walking_scopes};
use crate::nameres::namespaces::NAMES;
use crate::nameres::path_resolution::get_method_resolve_variants;
use crate::nameres::scope::{ScopeEntryExt, ScopeEntryListExt, VecExt, into_field_shorthand_items};
use crate::node_ext::item_spec::ItemSpecExt;
use crate::node_ext::{any_field_ext, item_spec};
use crate::types::expectation::Expected;
use crate::types::inference::{InferenceCtx, TypeError};
use crate::types::patterns::{BindingMode, anonymous_pat_ty_var};
use crate::types::substitution::ApplySubstitution;
use crate::types::ty::Ty;
use crate::types::ty::integer::IntegerKind;
use crate::types::ty::range_like::TySequence;
use crate::types::ty::reference::{Mutability, autoborrow};
use crate::types::ty::ty_callable::{TyCallable, TyCallableKind};
use crate::types::ty::ty_var::{TyInfer, TyIntVar};
use crate::types::ty_db;
use itertools::Itertools;
use std::iter;
use std::ops::Deref;
use syntax::ast::HasStmts;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, IntoNodeOrToken, ast, pretty_print};

pub struct TypeAstWalker<'a, 'db> {
    pub ctx: &'a mut InferenceCtx<'db>,
    pub expected_return_ty: Ty,
}

impl<'a, 'db> TypeAstWalker<'a, 'db> {
    pub fn new(ctx: &'a mut InferenceCtx<'db>, expected_return_ty: Ty) -> Self {
        TypeAstWalker { ctx, expected_return_ty }
    }

    pub fn walk(&mut self, ctx_owner: ast::InferenceCtxOwner) {
        self.collect_parameter_bindings(&ctx_owner);

        match ctx_owner {
            ast::InferenceCtxOwner::Fun(fun) => {
                if let Some(fun_block_expr) = fun.body() {
                    self.infer_block_expr(
                        &fun_block_expr,
                        Expected::ExpectType(self.expected_return_ty.clone()),
                        true,
                    );
                }
            }
            ast::InferenceCtxOwner::SpecFun(spec_fun) => {
                if let Some(spec_block_expr) = spec_fun.spec_block() {
                    self.process_msl_block_expr(
                        &spec_block_expr,
                        Expected::ExpectType(self.expected_return_ty.clone()),
                        true,
                    );
                }
            }
            ast::InferenceCtxOwner::SpecInlineFun(spec_fun) => {
                if let Some(spec_block_expr) = spec_fun.spec_block() {
                    self.process_msl_block_expr(
                        &spec_block_expr,
                        Expected::ExpectType(self.expected_return_ty.clone()),
                        true,
                    );
                }
            }
            ast::InferenceCtxOwner::ItemSpec(item_spec) => {
                if let Some(block_expr) = item_spec.spec_block() {
                    self.process_msl_block_expr(&block_expr, Expected::NoValue, false);
                }
            }
            ast::InferenceCtxOwner::Schema(schema) => {
                if let Some(block_expr) = schema.spec_block() {
                    self.process_msl_block_expr(&block_expr, Expected::NoValue, false);
                }
            }
            ast::InferenceCtxOwner::Initializer(initializer) => {
                let initializer_owner = initializer
                    .syntax()
                    .parent_of_type::<ast::InitializerOwner>()
                    .unwrap();
                let expected_ty = match initializer_owner {
                    ast::InitializerOwner::Const(const_) => {
                        ty_db::lower_type_owner_for_ctx(self.ctx, const_.in_file(self.ctx.file_id))
                    }
                    ast::InitializerOwner::AttrItem(attr_item) => {
                        attr_item.is_abort_code().then_some(Ty::Integer(IntegerKind::U64))
                    }
                };
                if let Some(expr) = initializer.expr() {
                    self.infer_expr_coerceable_to(&expr, expected_ty.unwrap_or(Ty::Unknown));
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
                .map(|it| it.ret_type_ty());
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
                self.collect_item_spec_signature_bindings(item_spec, item.clone());
                binding_file_id = item.file_id;
                match item.value {
                    ast::ItemSpecItem::Fun(fun) => fun.params_as_bindings(),
                    _ => vec![],
                }
            }
            _ => {
                return None;
            }
        };
        for binding in bindings {
            let binding_ty = {
                let binding_type_owner = binding.ident_owner();
                match binding_type_owner {
                    Some(ast::IdentPatOwner::Param(fun_param)) => fun_param
                        .type_()
                        .map(|it| ty_db::lower_type_for_ctx(self.ctx, it.in_file(binding_file_id)))
                        .unwrap_or(Ty::Unknown),
                    _ => continue,
                }
            };
            self.ctx.pat_types.insert(binding.into(), binding_ty);
        }
        Some(())
    }

    pub fn process_msl_block_expr(
        &mut self,
        block_expr: &ast::BlockExpr,
        expected_return: Expected,
        check_return_type: bool,
    ) -> Option<()> {
        self.ctx.msl_scope(|ctx| {
            let mut w = TypeAstWalker::new(ctx, Ty::Unit);
            w.infer_block_expr(&block_expr, expected_return, check_return_type);
        });
        Some(())
    }

    pub fn infer_block_expr(
        &mut self,
        block_expr: &ast::BlockExpr,
        expected_return: Expected,
        coerce_return_type: bool,
    ) -> Ty {
        let mut stmts = block_expr.stmts().collect::<Vec<_>>();
        // process let stmts first for msl
        if self.ctx.msl {
            let let_stmts = stmts.extract_if(.., |it| it.syntax().is::<ast::LetStmt>());
            for let_stmt in let_stmts {
                self.process_stmt(let_stmt);
            }
        }
        for stmt in stmts {
            self.process_stmt(stmt);
        }
        let tail_expr = block_expr.tail_expr();
        let opt_expected_ty = expected_return.ty(self.ctx);
        match tail_expr {
            None => {
                if let Some(expected_ty) = opt_expected_ty {
                    let error_target = block_expr
                        .r_curly_token()
                        .map(|it| it.into())
                        .unwrap_or(block_expr.node_or_token());
                    if coerce_return_type {
                        self.ctx.coerce_types(error_target, Ty::Unit, expected_ty);
                    } else {
                        let _ = self.ctx.combine_types(Ty::Unit, expected_ty);
                    }
                }
                Ty::Unit
            }
            Some(tail_expr) => {
                let expected = Expected::from_ty(opt_expected_ty);
                if coerce_return_type {
                    self.infer_expr_coerce_to(&tail_expr, expected)
                } else {
                    self.infer_expr(&tail_expr, expected)
                }
            }
        }
    }

    fn process_stmt(&mut self, stmt: ast::Stmt) -> Option<()> {
        let file_id = self.ctx.file_id;
        match stmt {
            ast::Stmt::LetStmt(let_stmt) => {
                let explicit_ty = let_stmt
                    .type_()
                    .map(|it| ty_db::lower_type_for_ctx(self.ctx, it.in_file(file_id)));
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
                        .map(|it| anonymous_pat_ty_var(&self.ctx.ty_var_index, &it))
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
            ast::Stmt::IncludeSchema(include_schema) => {
                self.process_include_schema(&include_schema);
            }
            ast::Stmt::AbortsIfStmt(aborts_if_stmt) => {
                self.process_aborts_if_stmt(&aborts_if_stmt);
            }
            ast::Stmt::AbortsWithStmt(aborts_with_stmt) => {
                for expr in aborts_with_stmt.exprs() {
                    self.infer_expr_coerceable_to(&expr, Ty::Num);
                }
            }
            ast::Stmt::PragmaStmt(pragma_stmt) => {
                for attr_item in pragma_stmt.attr_items() {
                    if let Some(expr) = attr_item.expr() {
                        self.infer_expr(&expr, Expected::NoValue);
                    }
                }
            }
            ast::Stmt::GenericSpecStmt(generic_spec_stmt) => {
                if let Some(expr) = generic_spec_stmt.expr() {
                    self.infer_expr_coerceable_to(&expr, Ty::Bool);
                }
            }
            ast::Stmt::SchemaField(schema_field) => {
                if let Some(ident_pat) = schema_field.ident_pat() {
                    let ty = ty_db::lower_type_owner_for_ctx(
                        self.ctx,
                        schema_field.in_file(self.ctx.file_id),
                    )
                    .unwrap_or(Ty::Unknown);
                    self.collect_pat_bindings(ident_pat.into(), ty, BindingMode::BindByValue);
                }
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
    fn infer_expr_coerce_to(&mut self, expr: &ast::Expr, expected: Expected) -> Ty {
        let actual_ty = self.infer_expr(expr, expected.clone());
        let Some(expected_ty) = expected.ty(&self.ctx) else {
            return actual_ty;
        };
        let no_type_error =
            self.ctx
                .coerce_types(expr.node_or_token(), actual_ty.clone(), expected_ty.clone());
        if no_type_error { expected_ty } else { actual_ty }
    }

    fn infer_expr(&mut self, expr: &ast::Expr, expected: Expected) -> Ty {
        self.ctx.db.unwind_if_revision_cancelled();

        if self.ctx.expr_types.contains_key(expr) {
            let file_text = expr
                .syntax()
                .containing_file()
                .unwrap()
                .syntax()
                .text()
                .to_string();
            let file = pretty_print::underline_range_in_text(&file_text, expr.syntax().text_range());
            unreachable!("trying to infer expr twice, \n{}", file);
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

            ast::Expr::CallExpr(call_expr) => {
                self.infer_call_expr(call_expr, expected).unwrap_or(Ty::Unknown)
            }

            ast::Expr::MethodCallExpr(method_call_expr) => {
                self.infer_method_call_expr(method_call_expr, expected)
            }
            ast::Expr::VectorLitExpr(vector_lit_expr) => {
                self.infer_vector_lit_expr(vector_lit_expr, expected)
            }
            ast::Expr::TupleExpr(tuple_expr) => self.infer_tuple_expr(tuple_expr, expected),
            ast::Expr::RangeExpr(range_expr) => {
                self.infer_range_expr(range_expr, expected).unwrap_or(Ty::Unknown)
            }
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
                if let Some(inner_expr) = return_expr.expr() {
                    self.infer_expr_coerceable_to(&inner_expr, self.expected_return_ty.clone());
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
                    self.infer_expr_coerceable_to(&inner_expr, Ty::Integer(IntegerKind::U64));
                }
                Ty::Never
            }

            ast::Expr::BlockExpr(block_expr) => self.infer_block_expr(block_expr, expected, true),
            ast::Expr::BinExpr(bin_expr) => self.infer_bin_expr(bin_expr).unwrap_or(Ty::Unknown),

            ast::Expr::BangExpr(bang_expr) => bang_expr
                .expr()
                .map(|it| {
                    self.infer_expr(&it, Expected::ExpectType(Ty::Bool));
                    Ty::Bool
                })
                .unwrap_or(Ty::Unknown),

            ast::Expr::MinusExpr(minus_expr) => self.infer_minus_expr(minus_expr).unwrap_or(Ty::Unknown),

            ast::Expr::Literal(lit) => self.infer_literal(lit),
            ast::Expr::UnitExpr(_) => Ty::Unit,
            ast::Expr::AnnotatedExpr(annotated_expr) => {
                self.infer_annotated_expr(annotated_expr).unwrap_or(Ty::Unknown)
            }

            ast::Expr::ForallExpr(it) => self.infer_quant_expr(&it.clone().into()).unwrap_or(Ty::Bool),
            ast::Expr::ExistsExpr(it) => self.infer_quant_expr(&it.clone().into()).unwrap_or(Ty::Bool),
            ast::Expr::ChooseExpr(it) => self.infer_choose_expr(&it),

            ast::Expr::SpecBlockExpr(it) => {
                if let Some(block_expr) = it.block_expr() {
                    self.process_msl_block_expr(&block_expr, Expected::NoValue, false);
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

        if self.ctx.msl {
            if let Some(path_expr_ty) = item_spec::infer_special_path_expr_for_item_spec(
                self.ctx.db,
                path_expr.in_file(self.ctx.file_id),
            ) {
                self.ctx
                    .expr_types
                    .insert(path_expr.to_owned().into(), path_expr_ty.clone());
                return Some(path_expr_ty);
            }
        }

        let expected_ty = expected.ty(self.ctx);
        let named_elements = self.ctx.resolve_path_cached_multi(path_expr.path(), expected_ty);
        let named_element = if named_elements.len() == 2 {
            // if it's field and ident pat, fetch type of ident_pat
            let (_, ident_pat) = into_field_shorthand_items(self.ctx.db, named_elements.clone())?;
            ident_pat.map_into()
        } else {
            named_elements
                .single_or_none()?
                .cast_into::<ast::NamedElement>(self.ctx.db)?
        };

        match named_element.kind() {
            IDENT_PAT => {
                let ident_pat = named_element.cast_into::<ast::IdentPat>()?.value;
                self.ctx.get_binding_type(ident_pat)
            }
            CONST => {
                let const_type = named_element
                    .cast_into::<ast::Const>()?
                    .and_then(|it| it.type_())?;
                Some(ty_db::lower_type_for_ctx(self.ctx, const_type))
            }
            NAMED_FIELD => {
                let named_field = named_element.cast_into::<ast::NamedField>()?;
                ty_db::lower_type_owner_for_ctx(self.ctx, named_field)
            }
            STRUCT | ENUM => {
                let struct_or_enum = named_element.cast_into::<ast::StructOrEnum>().unwrap();
                let adt_ty = self
                    .ctx
                    .instantiate_path_with_ty_vars(path_expr.path().into(), struct_or_enum);
                Some(adt_ty)
            }
            VARIANT => {
                // MyEnum::MyVariant
                let variant = named_element.cast_into::<ast::Variant>().unwrap();
                let enum_path = path_expr.path().qualifier().unwrap_or(path_expr.path());
                let variant_ty = self
                    .ctx
                    .instantiate_path_with_ty_vars(enum_path.into(), variant.map(|it| it.enum_()));
                let variant_ty_adt = variant_ty.into_ty_adt()?;
                Some(Ty::Adt(variant_ty_adt))
            }
            MODULE => None,
            FUN | SPEC_FUN | SPEC_INLINE_FUN => {
                let any_fun = named_element.cast_into::<ast::AnyFun>().unwrap();
                let method_or_path: ast::MethodOrPath = path_expr.path().into();
                Some(self.ctx.instantiate_path_for_fun(method_or_path, any_fun).into())
            }
            GLOBAL_VARIABLE_DECL => {
                let global_variable_decl = named_element.cast_into::<ast::GlobalVariableDecl>()?;
                ty_db::lower_type_owner_for_ctx(self.ctx, global_variable_decl)
            }
            _ => None,
        }
    }

    fn infer_dot_expr(&mut self, dot_expr: &ast::DotExpr, _expected: Expected) -> Option<Ty> {
        let self_ty = self.infer_expr(&dot_expr.receiver_expr(), Expected::NoValue);
        let self_ty = self.ctx.resolve_ty_vars_if_possible(self_ty);

        let ty_adt = self_ty.unwrap_all_refs().into_ty_adt()?;

        let field_name_ref = dot_expr.name_ref()?;
        let adt_item = ty_adt.adt_item_loc.to_ast::<ast::StructOrEnum>(self.ctx.db)?;
        let field_reference_name = field_name_ref.as_string();

        if !self.ctx.msl {
            let context = dot_expr.receiver_expr().in_file(self.ctx.file_id);
            // check for struct/enum visibility
            if let Some(adt_item_entry) = adt_item.clone().to_entry()
                && is_visible_in_context(self.ctx.db, &adt_item_entry, context.syntax()).is_some()
            {
                return None;
            }
        }
        //
        // if !self.ctx.msl
        //     && field_name_ref.syntax().containing_module() != ty_adt.adt_item_module(self.ctx.db)
        // {
        //     return None;
        // }

        // todo: tuple index fields

        let InFile {
            file_id: item_file_id,
            value: adt_item,
        } = adt_item;

        let matching_field = adt_item
            .fields()
            .into_iter()
            .filter(|(name, _)| name == &field_reference_name)
            .collect::<Vec<_>>()
            .single_or_none();

        self.ctx.resolved_fields.insert(
            field_name_ref,
            matching_field.clone().and_then(|(name, any_field)| {
                any_field_ext::to_scope_entry(name, item_file_id, any_field)
            }),
        );

        let field_type = matching_field?.1.type_()?.in_file(item_file_id);
        let field_ty = ty_db::lower_type_for_ctx(self.ctx, field_type).substitute(&ty_adt.substitution);
        Some(field_ty)
    }

    fn infer_method_call_expr(
        &mut self,
        method_call_expr: &ast::MethodCallExpr,
        expected: Expected,
    ) -> Ty {
        let self_ty = self.infer_expr(&method_call_expr.receiver_expr(), Expected::NoValue);
        let self_ty = self.ctx.resolve_ty_vars_if_possible(self_ty);

        let vis_ctx = InFile::new(self.ctx.file_id, method_call_expr.syntax().clone());
        let method_ref_name = method_call_expr.reference_name();
        let method_entry =
            get_method_resolve_variants(self.ctx.db, &self_ty, self.ctx.file_id, self.ctx.msl)
                .into_iter()
                .filter(|e| e.name == method_ref_name)
                .filter(|e| is_visible_in_context(self.ctx.db, e, vis_ctx.clone()).is_none())
                .exactly_one()
                .ok();
        self.ctx
            .resolved_method_calls
            .insert(method_call_expr.to_owned(), method_entry.clone());

        let resolved_method = method_entry.and_then(|it| it.node_loc.to_ast::<ast::Fun>(self.ctx.db));
        let method_ty = match resolved_method {
            Some(method) => self
                .ctx
                .instantiate_path_for_fun(method_call_expr.to_owned().into(), method.map_into()),
            None => {
                // add 1 for `self` parameter
                TyCallable::fake(1 + method_call_expr.arg_exprs().len(), TyCallableKind::fake())
            }
        };
        let method_call_ty = self.ctx.resolve_ty_vars_if_possible(method_ty);

        let expected_arg_tys = self.infer_expected_call_arg_tys(&method_call_ty, expected);
        let args = iter::once(CallArg::Self_ { self_ty })
            .chain(
                method_call_expr
                    .arg_exprs()
                    .into_iter()
                    .map(|arg_expr| CallArg::Arg { expr: arg_expr }),
            )
            .collect();
        self.coerce_call_arg_types(args, method_call_ty.param_types.clone(), expected_arg_tys);

        self.ctx
            .call_expr_types
            .insert(method_call_expr.clone().into(), method_call_ty.clone().into());

        method_call_ty.ret_type_ty()
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
            _ => TyCallable::fake(call_expr.arg_exprs().len(), TyCallableKind::fake()),
        };
        let expected_arg_tys = self.infer_expected_call_arg_tys(&callable_ty, expected);
        let args = call_expr
            .arg_exprs()
            .into_iter()
            .map(|expr| CallArg::Arg { expr })
            .collect();
        self.coerce_call_arg_types(args, callable_ty.param_types.clone(), expected_arg_tys);

        self.ctx
            .call_expr_types
            .insert(call_expr.clone().into(), callable_ty.clone().into());

        // resolve after applying all parameters
        let ret_ty = self.ctx.resolve_ty_vars_if_possible(callable_ty.ret_type_ty());
        Some(ret_ty)
    }

    fn infer_assert_macro_expr(&mut self, assert_macro_expr: &ast::AssertMacroExpr) -> Ty {
        let declared_input_tys = vec![Ty::Bool, Ty::Integer(IntegerKind::U64)];
        let args = assert_macro_expr
            .arg_exprs()
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

        let fields_owner = item.syntax().cast::<ast::FieldsOwner>();
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
        let explicit_ty_adt = self
            .ctx
            .instantiate_path_with_ty_vars(path.into(), struct_or_enum.in_file(item_file_id))
            .into_ty_adt()?;

        let mut ty_adt = explicit_ty_adt.clone();
        if let Some(Ty::Adt(expected_ty_adt)) = expected_ty {
            let expected_subst = expected_ty_adt.substitution;
            for (type_param, explicit_subst_ty) in explicit_ty_adt.substitution.entries() {
                // skip type parameters as we have no ability check
                if matches!(explicit_subst_ty, &Ty::TypeParam(_)) {
                    continue;
                }
                if let Some(expected_subst_ty) = expected_subst.get_ty(&type_param) {
                    // unifies if `substTy` is TyVar, performs type check if `substTy` is real type
                    let _ = self
                        .ctx
                        .combine_types(explicit_subst_ty.to_owned(), expected_subst_ty);
                }
            }
            // resolved tyAdt inner TyVars after combining with expectedTy
            ty_adt = self.ctx.resolve_ty_vars_if_possible(explicit_ty_adt);
        }

        let named_fields = fields_owner.named_fields_map();
        for lit_field in struct_lit.fields() {
            let lit_field_name = lit_field.field_name_ref();
            if lit_field_name.is_none() {
                continue;
            }
            let lit_field_name = lit_field_name.unwrap().as_string();

            let named_field = named_fields.get(&lit_field_name);
            let declared_field_ty = named_field
                .and_then(|field| {
                    ty_db::lower_type_owner_for_ctx(self.ctx, field.to_owned().in_file(item_file_id))
                })
                .unwrap_or(Ty::Unknown);
            let field_ty = declared_field_ty.substitute(&ty_adt.substitution);

            if let Some(lit_field_expr) = lit_field.expr() {
                self.infer_expr_coerceable_to(&lit_field_expr, field_ty);
            } else {
                let walk_ctx = WalkScopesCtx {
                    start_at: lit_field.syntax().clone().in_file(self.ctx.file_id),
                    allowed_ns: NAMES,
                    expected_name: Some(lit_field_name.clone()),
                };
                let binding = get_entries_from_walking_scopes(self.ctx.db, walk_ctx)
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

        // resolve after processing all fields
        let struct_lit_ty = self.ctx.resolve_ty_vars_if_possible(Ty::Adt(ty_adt));

        Some(struct_lit_ty)
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
                        if self.ctx.msl { Ty::Num } else { Ty::integer() },
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

    fn infer_range_expr(&mut self, range_expr: &ast::RangeExpr, _expected: Expected) -> Option<Ty> {
        let start_expr = range_expr.start_expr()?;
        let start_ty = self.infer_expr(&start_expr, Expected::NoValue);
        if let Some(end_expr) = range_expr.end_expr() {
            self.infer_expr_coerceable_to(&end_expr, start_ty.clone());
        }
        Some(Ty::Seq(TySequence::Range(Box::new(start_ty))))
    }

    fn infer_vector_lit_expr(&mut self, vector_lit_expr: &ast::VectorLitExpr, expected: Expected) -> Ty {
        let arg_ty_var = Ty::new_ty_var(&self.ctx.ty_var_index);

        let explicit_type = vector_lit_expr.type_arg().and_then(|it| it.type_());
        if let Some(explicit_type) = explicit_type {
            let explicit_ty =
                ty_db::lower_type_for_ctx(self.ctx, explicit_type.in_file(self.ctx.file_id));
            let _ = self.ctx.combine_types(arg_ty_var.clone(), explicit_ty);
        }

        let arg_ty = self.ctx.resolve_ty_vars_if_possible(arg_ty_var.clone());
        let arg_exprs = vector_lit_expr.arg_exprs().collect::<Vec<_>>();
        let declared_arg_tys = iter::repeat_n(arg_ty, arg_exprs.len()).collect::<Vec<_>>();

        let vec_ty = Ty::new_vector(arg_ty_var);

        let lit_call_ty =
            TyCallable::new(declared_arg_tys.clone(), vec_ty.clone(), TyCallableKind::fake());
        let expected_arg_tys = self.infer_expected_call_arg_tys(&lit_call_ty, expected);
        let args = arg_exprs
            .into_iter()
            .map(|expr| CallArg::Arg { expr: Some(expr) })
            .collect();
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
        let condition_expr = if_expr.condition_expr()?;
        self.infer_expr_coerceable_to(&condition_expr, Ty::Bool);

        let actual_if_ty = if_expr
            .then_branch()
            .map(|it| self.infer_block_or_inline_expr(&it, expected.clone(), false));
        let Some(else_branch) = if_expr.else_branch() else {
            return Some(Ty::Unit);
        };
        let actual_else_ty = self.infer_block_or_inline_expr(&else_branch, expected.clone(), false);

        // try comparing branch types to each other, if they are compatible, then the whole if-else expr is wrong
        let branches_compat = self
            .ctx
            .combine_types(
                actual_else_ty.unwrap_all_refs(),
                actual_if_ty.clone().unwrap_or(Ty::Unknown).unwrap_all_refs(),
            )
            .is_ok();

        let expected_ty = expected.ty(&self.ctx);
        if !branches_compat {
            // if not compatible to each other, then:
            // 1. if expected type is present, check both separately against it
            if let Some(expected_ty) = expected_ty {
                if let Some(if_ty) = actual_if_ty {
                    self.coerce_if_else_branch(if_expr.then_branch(), if_ty, expected_ty.clone());
                }
                self.coerce_if_else_branch(if_expr.else_branch(), actual_else_ty, expected_ty);
            } else {
                // 2. not present -> check that else is compatible with if (ignoring refs)
                self.coerce_if_else_branch(
                    if_expr.else_branch(),
                    actual_else_ty.unwrap_all_refs(),
                    actual_if_ty.unwrap_or(Ty::Unknown).unwrap_all_refs(),
                );
            }
            return Some(Ty::Unknown);
        }

        // special-case:
        //      let a: &mut R = if (true) &R else &mut R;
        // we want to highlight only else branch here
        if branches_compat
            && expected_ty
                .clone()
                .is_some_and(|it| matches!(it, Ty::Reference(_)))
        {
            let expected_ty = expected_ty.unwrap();
            if let Some(if_ty) = actual_if_ty.clone() {
                self.coerce_if_else_branch(if_expr.then_branch(), if_ty, expected_ty.clone());
            }
            self.coerce_if_else_branch(if_expr.else_branch(), actual_else_ty.clone(), expected_ty);
        }

        // if branches are compatible, then we need to check returning type of if-else with expected type
        let tys = vec![actual_if_ty, Some(actual_else_ty)]
            .into_iter()
            .filter_map(|it| it)
            .collect();
        Some(self.ctx.intersect_all_types(tys))
    }

    fn coerce_if_else_branch(
        &mut self,
        branch: Option<ast::BlockOrInlineExpr>,
        actual_ty: Ty,
        expected_ty: Ty,
    ) {
        if let Some(branch) = branch {
            if let Some(tail_node_or_token) = branch.tail_node_or_token() {
                self.ctx.coerce_types(tail_node_or_token, actual_ty, expected_ty);
            }
        }
    }

    fn infer_loop_expr(&mut self, loop_expr: &ast::LoopExpr) -> Ty {
        self.infer_loop_like_body(loop_expr.clone().into())
    }

    fn infer_while_expr(&mut self, while_expr: &ast::WhileExpr) -> Ty {
        let condition_expr = while_expr.condition().and_then(|it| it.expr());
        if let Some(condition_expr) = condition_expr {
            self.infer_expr_coerceable_to(&condition_expr, Ty::Bool);
        }
        self.infer_loop_like_body(while_expr.clone().into())
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
        self.infer_loop_like_body(for_expr.clone().into())
    }

    fn infer_loop_like_body(&mut self, loop_like: ast::LoopLike) -> Ty {
        if let Some(loop_body_expr) = loop_like.loop_body_expr() {
            self.infer_block_or_inline_expr(&loop_body_expr, Expected::ExpectType(Ty::Unit), false);
        }
        Ty::Never
    }

    fn infer_cast_expr(&mut self, cast_expr: &ast::CastExpr, expected: Expected) -> Ty {
        self.infer_expr(&cast_expr.expr(), Expected::NoValue);
        if let Some(type_) = cast_expr.type_() {
            let ty = ty_db::lower_type_for_ctx(self.ctx, type_.in_file(self.ctx.file_id));
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
        check_return_type: bool,
    ) -> Ty {
        match block_or_inline_expr {
            ast::BlockOrInlineExpr::BlockExpr(block_expr) => {
                self.infer_block_expr(block_expr, expected, check_return_type)
            }
            ast::BlockOrInlineExpr::InlineExpr(inline_expr) => inline_expr
                .expr()
                .map(|expr| self.infer_expr(&expr, expected))
                .unwrap_or(Ty::Unknown),
        }
    }

    fn infer_borrow_expr(&mut self, borrow_expr: &ast::BorrowExpr, expected: Expected) -> Option<Ty> {
        let inner_expr = borrow_expr.expr()?;
        let expected_inner_ty = expected
            .ty(self.ctx)
            .and_then(|ty| ty.into_ty_ref())
            .map(|ty_ref| ty_ref.referenced());

        let inner_ty = self.infer_expr(&inner_expr, Expected::from_ty(expected_inner_ty));
        let inner_ty = match inner_ty {
            Ty::Reference(_) | Ty::Tuple(_) => {
                self.ctx
                    .type_errors
                    .push(TypeError::wrong_arguments_to_borrow_expr(inner_expr, inner_ty));
                Ty::Unknown
            }
            _ => inner_ty,
        };
        let mutability = Mutability::new(borrow_expr.is_mut());

        Some(Ty::new_reference(inner_ty, mutability))
    }

    fn infer_deref_expr(&mut self, deref_expr: &ast::DerefExpr, _expected: Expected) -> Option<Ty> {
        let inner_expr = deref_expr.expr()?;

        let inner_ty = self.infer_expr(&inner_expr, Expected::NoValue);
        let inner_ty = self.ctx.resolve_ty_vars_if_possible(inner_ty);
        let inner_ty = match inner_ty {
            Ty::Reference(_) => inner_ty,
            _ => {
                self.ctx
                    .type_errors
                    .push(TypeError::invalid_dereference(inner_expr, inner_ty));
                return None;
            }
        };
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
            ast::BinaryOp::ArithOp(arith_op) => {
                self.infer_arith_binary_expr(bin_expr, lhs, rhs, arith_op, false)
            }

            ast::BinaryOp::Assignment { op: None } => self.infer_assignment(lhs, rhs),
            ast::BinaryOp::Assignment { op: Some(arith_op) } => {
                self.infer_arith_binary_expr(bin_expr, lhs, rhs, arith_op, true)
            }

            ast::BinaryOp::LogicOp(_) => self.infer_logic_binary_expr(lhs, rhs),

            ast::BinaryOp::CmpOp(op) => match op {
                ast::CmpOp::Eq { .. } => self.infer_eq_binary_expr(bin_expr, lhs, rhs, op),
                ast::CmpOp::Ord { .. } => self.infer_ordering_binary_expr(lhs, rhs, op),
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
        bin_expr: &ast::BinExpr,
        lhs: ast::Expr,
        rhs: Option<ast::Expr>,
        arith_op: ast::ArithOp,
        is_compound: bool,
    ) -> Ty {
        if matches!(
            arith_op,
            ast::ArithOp::BitAnd | ast::ArithOp::BitOr | ast::ArithOp::BitXor
        ) {
            return self.infer_bit_ops_binary_expr(lhs, rhs, is_compound);
        }
        if matches!(arith_op, ast::ArithOp::Shl | ast::ArithOp::Shr) {
            return self.infer_bit_shifts_binary_expr(lhs, rhs, is_compound);
        }

        let mut is_error = false;
        let left_ty = self.infer_expr(&lhs, Expected::NoValue);
        if !self
            .ctx
            .resolve_ty_vars_if_possible(left_ty.clone())
            .supports_arithm_op()
        {
            self.ctx.push_type_error(TypeError::unsupported_op(
                &lhs,
                left_ty.clone(),
                ast::BinaryOp::ArithOp(arith_op),
            ));
            is_error = true;
        }
        if let Some(rhs) = rhs {
            let right_ty = self.infer_expr(&rhs, Expected::ExpectType(left_ty.clone()));
            if !self
                .ctx
                .resolve_ty_vars_if_possible(right_ty.clone())
                .supports_arithm_op()
            {
                self.ctx.push_type_error(TypeError::unsupported_op(
                    &rhs,
                    right_ty.clone(),
                    ast::BinaryOp::ArithOp(arith_op),
                ));
                is_error = true;
            }
            if !is_error {
                let combined = self.ctx.combine_types(left_ty.clone(), right_ty.clone());
                if combined.is_err() {
                    self.ctx.push_type_error(TypeError::wrong_arguments_to_bin_expr(
                        bin_expr.clone(),
                        left_ty.clone(),
                        right_ty,
                        ast::BinaryOp::ArithOp(arith_op),
                    ));
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

    fn infer_bit_ops_binary_expr(
        &mut self,
        lhs: ast::Expr,
        rhs: Option<ast::Expr>,
        is_compound: bool,
    ) -> Ty {
        let lhs_ty = self.infer_expr_coerceable_to(&lhs, Ty::integer());
        if let Some(rhs) = rhs {
            self.infer_expr_coerceable_to(&rhs, lhs_ty.clone());
        }
        if is_compound { Ty::Unit } else { lhs_ty }
    }

    fn infer_bit_shifts_binary_expr(
        &mut self,
        lhs: ast::Expr,
        rhs: Option<ast::Expr>,
        is_compound: bool,
    ) -> Ty {
        let lhs_ty = self.infer_expr_coerceable_to(&lhs, Ty::integer());
        if let Some(rhs) = rhs {
            self.infer_expr_coerceable_to(&rhs, Ty::Integer(IntegerKind::U8));
        }
        if is_compound { Ty::Unit } else { lhs_ty }
    }

    fn infer_logic_binary_expr(&mut self, lhs: ast::Expr, rhs: Option<ast::Expr>) -> Ty {
        self.infer_expr_coerceable_to(&lhs, Ty::Bool);
        if let Some(rhs) = rhs {
            self.infer_expr_coerceable_to(&rhs, Ty::Bool);
        }
        Ty::Bool
    }

    fn infer_eq_binary_expr(
        &mut self,
        bin_expr: &ast::BinExpr,
        lhs: ast::Expr,
        rhs: Option<ast::Expr>,
        cmp_op: ast::CmpOp,
    ) -> Ty {
        let left_ty = self.infer_expr(&lhs, Expected::NoValue);
        let mut left_ty = self.ctx.resolve_ty_vars_if_possible(left_ty);

        if let Some(rhs) = rhs {
            let right_ty = self.infer_expr(&rhs, Expected::NoValue);
            let mut right_ty = self.ctx.resolve_ty_vars_if_possible(right_ty);

            // equality should ignore references
            if let (Ty::Reference(left_ref_ty), Ty::Reference(right_ref_ty)) = (&left_ty, &right_ty) {
                left_ty = left_ref_ty.referenced();
                right_ty = right_ref_ty.referenced();
            }

            let combined = self.ctx.combine_types(left_ty.clone(), right_ty.clone());
            if combined.is_err() {
                self.ctx.push_type_error(TypeError::wrong_arguments_to_bin_expr(
                    bin_expr.clone(),
                    left_ty,
                    right_ty,
                    ast::BinaryOp::CmpOp(cmp_op),
                ));
            }
        }
        Ty::Bool
    }

    fn infer_ordering_binary_expr(
        &mut self,
        lhs: ast::Expr,
        rhs: Option<ast::Expr>,
        cmp_op: ast::CmpOp,
    ) -> Ty {
        let mut is_error = false;
        let left_ty = self.infer_expr(&lhs, Expected::NoValue);
        if !self
            .ctx
            .resolve_ty_vars_if_possible(left_ty.clone())
            .supports_ordering()
        {
            self.ctx.push_type_error(TypeError::unsupported_op(
                &lhs,
                left_ty.clone(),
                ast::BinaryOp::CmpOp(cmp_op),
            ));
            is_error = true;
        }
        if let Some(rhs) = rhs {
            let right_ty = self.infer_expr(&rhs, Expected::NoValue);
            if !self
                .ctx
                .resolve_ty_vars_if_possible(right_ty.clone())
                .supports_ordering()
            {
                self.ctx.push_type_error(TypeError::unsupported_op(
                    &rhs,
                    right_ty.clone(),
                    ast::BinaryOp::CmpOp(cmp_op),
                ));
                is_error = true;
            }
            if !is_error {
                self.ctx.coerce_types(rhs.node_or_token(), right_ty, left_ty);
            }
        }
        Ty::Bool
    }

    fn infer_annotated_expr(&mut self, annotated_expr: &ast::AnnotatedExpr) -> Option<Ty> {
        let expr = annotated_expr.expr()?;

        let type_ = annotated_expr.type_()?;
        let ty = ty_db::lower_type_for_ctx(self.ctx, type_.in_file(self.ctx.file_id));

        let expr_ty = self.infer_expr_coerceable_to(&expr, ty);
        Some(expr_ty)
    }

    fn infer_minus_expr(&mut self, minus_expr: &ast::MinusExpr) -> Option<Ty> {
        minus_expr
            .expr()
            .map(|it| self.infer_expr_coerceable_to(&it, Ty::integer()))
    }

    fn infer_literal(&mut self, literal: &ast::Literal) -> Ty {
        match literal.kind() {
            ast::LiteralKind::Bool(_) => Ty::Bool,
            ast::LiteralKind::IntNumber(num) => {
                let kind = IntegerKind::from_suffixed_literal(num);
                match kind {
                    Some(kind) => Ty::Integer(kind),
                    None => Ty::Infer(TyInfer::IntVar(TyIntVar::new(self.ctx.ty_var_index.inc()))),
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
        let declared_ret_ty = self.ctx.resolve_ty_vars_if_possible(ty_callable.ret_type_ty());

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
        declared_param_tys: Vec<Ty>,
        expected_param_tys: Vec<Ty>,
    ) {
        for (i, declared_ty) in declared_param_tys.iter().enumerate() {
            let expected_ty = self.ctx.resolve_ty_vars_if_possible(
                expected_param_tys.get(i).unwrap_or(declared_ty).to_owned(),
            );
            let arg = args.get(i).cloned();
            match arg {
                Some(arg) => self.coerce_arg(arg, expected_ty),
                None => {
                    // missing parameter, combine expected type with Ty::Unknown
                    // let _ = self.ctx.combine_types(Ty::Unknown, expected_ty);
                }
            }
        }
        // extra arguments
        for arg in args.into_iter().skip(declared_param_tys.len()) {
            self.coerce_arg(arg, Ty::Unknown);
        }
    }

    fn coerce_arg(&mut self, arg: CallArg, expected_ty: Ty) {
        match arg {
            CallArg::Self_ { self_ty } => {
                let actual_self_ty = autoborrow(self_ty, &expected_ty)
                    .expect("method call won't be resolved if autoborrow fails");
                let _ = self.ctx.combine_types(actual_self_ty, expected_ty);
            }
            CallArg::Arg { expr } => {
                if let Some(expr) = expr {
                    let arg_expr_ty = self.infer_expr(&expr, Expected::ExpectType(expected_ty.clone()));
                    self.ctx
                        .coerce_types(expr.node_or_token(), arg_expr_ty, expected_ty);
                }
            }
        }
    }
}

#[derive(Clone)]
enum CallArg {
    Self_ { self_ty: Ty },
    Arg { expr: Option<ast::Expr> },
}
