// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

pub(crate) mod ast_walker;
pub(crate) mod combine_types;
pub(crate) mod inference_result;

use crate::nameres::binding::resolve_ident_pat_with_expected_type;
use crate::nameres::path_resolution;
use crate::nameres::scope::{ScopeEntry, VecExt};
use crate::types::fold::{Fallback, FullTyVarResolver, TyVarResolver, TypeFoldable};
use crate::types::has_type_params_ext::GenericItemExt;
use crate::types::substitution::ApplySubstitution;
use crate::types::ty::Ty;
use crate::types::ty::adt::TyAdt;
use crate::types::ty::ty_callable::{TyCallable, TyCallableKind};
use crate::types::ty::ty_var::{TyInfer, TyIntVar, TyVar};
use crate::types::unification::UnificationTable;
use base_db::SourceDatabase;
use std::cell::Cell;
use std::collections::HashMap;
use std::hash::Hash;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};
use vfs::FileId;

use crate::loc::SyntaxLocFileExt;
use crate::nameres::path_resolution::remove_variant_ident_pats;
use crate::types::ty_db;
pub use combine_types::TypeError;
use syntax::SyntaxKind::{STRUCT, VARIANT};
use syntax::ast::node_ext::struct_pat_field::PatFieldKind;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;

#[derive(Debug, Default)]
pub struct TyVarIndex(Cell<usize>);

impl TyVarIndex {
    pub fn inc(&self) -> usize {
        let new_val = self.0.get() + 1;
        self.0.set(new_val);
        new_val
    }
}

pub struct InferenceCtx<'db> {
    pub db: &'db dyn SourceDatabase,
    pub file_id: FileId,
    pub ty_var_index: TyVarIndex,
    pub msl: bool,

    pub type_errors: Vec<TypeError>,

    pub var_table: UnificationTable<TyVar>,
    pub int_table: UnificationTable<TyIntVar>,

    pub pat_types: HashMap<ast::Pat, Ty>,
    pub pat_field_types: HashMap<ast::StructPatField, Ty>,
    pub expr_types: HashMap<ast::Expr, Ty>,
    pub expected_expr_types: HashMap<ast::Expr, Ty>,

    pub call_expr_types: HashMap<ast::AnyCallExpr, TyCallable>,

    pub resolved_paths: HashMap<ast::Path, Vec<ScopeEntry>>,
    pub resolved_method_calls: HashMap<ast::MethodCallExpr, Option<ScopeEntry>>,
    pub resolved_fields: HashMap<ast::NameRef, Option<ScopeEntry>>,
    pub resolved_ident_pats: HashMap<ast::IdentPat, Option<ScopeEntry>>,

    pub lambda_exprs: Vec<ast::LambdaExpr>,
    pub lambda_expr_types: HashMap<ast::LambdaExpr, TyCallable>,
}

impl<'db> InferenceCtx<'db> {
    pub fn new(db: &'db dyn SourceDatabase, file_id: FileId, msl: bool) -> Self {
        InferenceCtx {
            db,
            file_id,
            ty_var_index: TyVarIndex::default(),
            msl,
            type_errors: vec![],
            var_table: UnificationTable::new(),
            int_table: UnificationTable::new(),
            expr_types: HashMap::new(),
            call_expr_types: HashMap::new(),
            expected_expr_types: HashMap::new(),
            pat_types: HashMap::new(),
            pat_field_types: HashMap::new(),
            resolved_paths: HashMap::new(),
            resolved_method_calls: HashMap::new(),
            resolved_fields: HashMap::new(),
            resolved_ident_pats: HashMap::new(),
            lambda_exprs: vec![],
            lambda_expr_types: HashMap::new(),
        }
    }

    pub fn from_parent_ctx(ctx: &'db InferenceCtx) -> InferenceCtx<'db> {
        let mut new_ctx = Self::new(ctx.db, ctx.file_id, ctx.msl);
        new_ctx.pat_types = ctx.pat_types.clone();
        new_ctx
    }

    pub fn resolve_path_cached(
        &mut self,
        path: ast::Path,
        expected_ty: Option<Ty>,
    ) -> Option<InFile<ast::NamedElement>> {
        self.resolve_path_cached_multi(path, expected_ty)
            .single_or_none()
            .and_then(|it| it.cast_into::<ast::NamedElement>(self.db))
    }

    pub fn resolve_path_cached_multi(
        &mut self,
        path: ast::Path,
        expected_ty: Option<Ty>,
    ) -> Vec<ScopeEntry> {
        let _p = tracing::debug_span!("resolve_path_cached_multi").entered();

        let path_entries =
            path_resolution::resolve_path(self.db, path.clone().in_file(self.file_id), expected_ty);

        let entries = remove_variant_ident_pats(self.db, path_entries, |ident_pat| {
            self.resolved_ident_pats
                .get(&ident_pat.value)
                .and_then(|it| it.clone())
        });
        self.resolved_paths.insert(path, entries.clone());

        entries
    }

    pub fn resolve_ident_pat_cached(
        &mut self,
        ident_pat: ast::IdentPat,
        expected_type: Option<Ty>,
    ) -> Option<InFile<ast::NamedElement>> {
        let entry = resolve_ident_pat_with_expected_type(
            self.db,
            ident_pat.clone().in_file(self.file_id),
            expected_type,
        );
        self.resolved_ident_pats.insert(ident_pat, entry.clone());

        entry.and_then(|it| it.cast_into::<ast::NamedElement>(self.db))
    }

    fn instantiate_adt_item_as_callable(
        &mut self,
        path: ast::Path,
        ty_adt: TyAdt,
    ) -> Option<TyCallable> {
        let adt_item = ty_adt.adt_item(self.db)?;
        let (adt_item_file_id, adt_item_value) = adt_item.clone().unpack();

        let resolved_to = self.resolved_paths.get(&path)?.clone().single_or_none()?;
        // if it's resolved to anything other than struct or enum variant, then it could only be a wrapped lambda
        if !matches!(resolved_to.kind(), STRUCT | VARIANT) {
            let wrapped_lambda_type = adt_item.and_then(|it| it.struct_()?.wrapped_lambda_type())?;
            let lambda_ty =
                ty_db::lower_type_for_ctx(self, wrapped_lambda_type.map_into()).into_ty_callable()?;
            return Some(lambda_ty);
        }

        let fields_owner: ast::FieldsOwner = match adt_item_value {
            ast::StructOrEnum::Struct(s) => s.into(),
            ast::StructOrEnum::Enum(_) => {
                // fetch variant
                let variant = resolved_to.cast_into::<ast::Variant>(self.db)?;
                variant.value.into()
            }
        };
        let fields_owner_loc = fields_owner.clone().in_file(adt_item_file_id).loc();

        let tuple_fields = fields_owner.tuple_field_list()?.fields().collect::<Vec<_>>();
        let param_types = tuple_fields
            .into_iter()
            .map(|it| {
                ty_db::lower_type_owner_for_ctx(self, it.in_file(adt_item_file_id))
                    .unwrap_or(Ty::Unknown)
            })
            .collect::<Vec<_>>();
        let ret_type = Ty::new_ty_adt(adt_item.clone());
        let callable_ty = TyCallable::new(
            param_types,
            ret_type,
            TyCallableKind::named(adt_item.ty_type_params_subst(), Some(fields_owner_loc)),
        )
        .substitute(&ty_adt.substitution);

        let ty_vars_subst = adt_item.ty_vars_subst(&self.ty_var_index);
        Some(callable_ty.substitute(&ty_vars_subst))
    }

    pub fn instantiate_path_for_fun(
        &mut self,
        method_or_path: ast::MethodOrPath,
        any_fun: InFile<ast::AnyFun>,
    ) -> TyCallable {
        let ty = self.instantiate_path_with_ty_vars(method_or_path, any_fun);
        match ty {
            Ty::Callable(ty_callable) => ty_callable,
            _ => unreachable!(
                "instantiate_path_for_fun() should return Ty::Callable, but returned {:?}",
                ty
            ),
        }
    }

    pub fn instantiate_path_with_ty_vars(
        &mut self,
        method_or_path: ast::MethodOrPath,
        generic_item: InFile<impl Into<ast::GenericElement>>,
    ) -> Ty {
        let generic_item = generic_item.map(|it| it.into());

        let ty_vars_subst = generic_item.ty_vars_subst(&self.ty_var_index);
        self.instantiate_path(method_or_path, generic_item.clone())
            .substitute(&ty_vars_subst)
    }

    pub fn instantiate_path(
        &mut self,
        method_or_path: ast::MethodOrPath,
        named_item: InFile<impl Into<ast::NamedElement>>,
    ) -> Ty {
        let (path_ty, ability_type_errors) = ty_db::lower_path(
            self.db,
            method_or_path.in_file(self.file_id),
            named_item,
            self.msl,
        );
        for ability_type_error in ability_type_errors {
            self.push_type_error(ability_type_error);
        }
        path_ty
    }

    pub fn resolve_ty_infer(&self, ty_infer: &TyInfer) -> Ty {
        match ty_infer {
            TyInfer::IntVar(ty_int_var) => self
                .int_table
                .resolve_to_ty_value(&ty_int_var)
                .unwrap_or(Ty::Infer(ty_infer.to_owned())),
            TyInfer::Var(ty_var) => {
                let var_value_ty = self.var_table.resolve_to_ty_value(ty_var);
                match &var_value_ty {
                    None => Ty::Infer(ty_infer.to_owned()),
                    Some(Ty::Infer(TyInfer::IntVar(int_var))) => self
                        .int_table
                        .resolve_to_ty_value(int_var)
                        .unwrap_or(var_value_ty.unwrap()),
                    Some(_) => var_value_ty.unwrap(),
                }
            }
        }
    }

    pub fn var_resolver(&self) -> TyVarResolver<'_> {
        TyVarResolver::new(self)
    }

    fn resolve_all_ty_vars_if_possible(&mut self) {
        self.expr_types = self.resolve_map_vars_if_possible(self.expr_types.clone());
        self.pat_types = self.resolve_map_vars_if_possible(self.pat_types.clone());
        self.lambda_expr_types = self.resolve_map_vars_if_possible(self.lambda_expr_types.clone());
    }

    pub fn resolve_map_vars_if_possible<Ast: Eq + Hash, T: TypeFoldable<T>>(
        &self,
        ty_map: HashMap<Ast, T>,
    ) -> HashMap<Ast, T> {
        ty_map
            .into_iter()
            .map(|(expr, ty)| (expr, self.resolve_ty_vars_if_possible(ty)))
            .collect()
    }

    pub fn resolve_ty_vars_if_possible<T: TypeFoldable<T>>(&self, ty: T) -> T {
        ty.fold_with(&self.var_resolver())
    }

    pub fn fully_resolve_vars<T: TypeFoldable<T>>(&self, foldable: T) -> T {
        foldable.fold_with(&FullTyVarResolver::new(&self, Fallback::Unknown))
    }

    pub fn fully_resolve_vars_fallback_to_origin<T: TypeFoldable<T>>(&self, foldable: T) -> T {
        foldable.fold_with(&FullTyVarResolver::new(&self, Fallback::Origin))
    }

    fn resolve_ty_infer_shallow(&self, ty: Ty) -> Ty {
        if let Ty::Infer(ty_infer) = &ty {
            self.resolve_ty_infer(ty_infer)
        } else {
            ty
        }
    }

    pub fn freeze<T>(&mut self, f: impl FnOnce(&mut InferenceCtx) -> T) -> T {
        self.var_table.snapshot();
        self.int_table.snapshot();
        let res = f(self);
        self.var_table.rollback();
        self.int_table.rollback();
        res
    }

    pub fn msl_scope<T>(&mut self, f: impl FnOnce(&mut InferenceCtx) -> T) -> T {
        if self.msl {
            return f(self);
        }
        self.msl = true;
        let res = self.freeze(|ctx| f(ctx));
        self.msl = false;
        res
    }

    pub fn get_binding_type(&self, ident_pat: ast::IdentPat) -> Option<Ty> {
        if let Some(pat_field) = ident_pat.syntax().parent_of_type::<ast::StructPatField>() {
            if matches!(pat_field.field_kind(), PatFieldKind::Shorthand { .. }) {
                return self.pat_field_types.get(&pat_field).map(|it| it.to_owned());
            }
        }
        self.pat_types.get(&ident_pat.into()).cloned()
    }

    pub fn push_type_error(&mut self, type_error: TypeError) {
        self.type_errors.push(type_error);
    }
}
