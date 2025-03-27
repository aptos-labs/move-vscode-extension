pub(crate) mod ast_walker;
pub(crate) mod combine_types;
pub(crate) mod inference_result;

use crate::InFile;
use crate::db::HirDatabase;
use crate::files::{InFileExt, InFileInto};
use crate::nameres::path_resolution;
use crate::nameres::scope::{ScopeEntry, VecExt};
use crate::types::fold::{Fallback, FullTyVarResolver, TyVarResolver, TypeFoldable};
use crate::types::has_type_params_ext::GenericItemExt;
use crate::types::lowering::TyLowering;
use crate::types::substitution::ApplySubstitution;
use crate::types::ty::Ty;
use crate::types::ty::ty_callable::{CallKind, TyCallable};
use crate::types::ty::ty_var::{TyInfer, TyIntVar, TyVar};
use crate::types::unification::UnificationTable;
use parser::SyntaxKind;
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::Deref;
use syntax::{AstNode, ast};
use vfs::FileId;

pub struct InferenceCtx<'db> {
    pub db: &'db dyn HirDatabase,
    pub file_id: FileId,
    pub ty_var_counter: usize,
    pub msl: bool,

    pub var_table: UnificationTable<TyVar>,
    pub int_table: UnificationTable<TyIntVar>,

    pub pat_types: HashMap<ast::Pat, Ty>,
    pub expr_types: HashMap<ast::Expr, Ty>,

    pub resolved_paths: HashMap<ast::Path, Vec<ScopeEntry>>,
    pub resolved_method_calls: HashMap<ast::MethodCallExpr, Option<ScopeEntry>>,
    pub resolved_fields: HashMap<ast::FieldRef, Option<ScopeEntry>>,

    pub lambda_exprs: Vec<ast::LambdaExpr>,
    pub lambda_expr_types: HashMap<ast::LambdaExpr, TyCallable>,
}

impl<'a> InferenceCtx<'a> {
    pub fn new(db: &'a dyn HirDatabase, file_id: FileId) -> Self {
        InferenceCtx {
            db,
            file_id,
            ty_var_counter: 0,
            msl: false,
            var_table: UnificationTable::new(),
            int_table: UnificationTable::new(),
            expr_types: HashMap::new(),
            pat_types: HashMap::new(),
            resolved_paths: HashMap::new(),
            resolved_method_calls: HashMap::new(),
            resolved_fields: HashMap::new(),
            lambda_exprs: vec![],
            lambda_expr_types: HashMap::new(),
        }
    }

    pub fn resolve_path_cached(
        &mut self,
        path: ast::Path,
        _expected_ty: Option<Ty>,
    ) -> Option<InFile<ast::AnyNamedElement>> {
        let entries = path_resolution::resolve_path(self.db, path.clone().in_file(self.file_id));
        self.resolved_paths.insert(path, entries.clone());

        entries
            .single_or_none()
            .and_then(|it| it.cast_into::<ast::AnyNamedElement>(self.db))
    }

    fn instantiate_call_expr_path(&mut self, call_expr: &ast::CallExpr) -> TyCallable {
        let path = call_expr.path();
        let named_item = self.resolve_path_cached(path.clone(), None);
        let callable_ty = if let Some(named_item) = named_item {
            let item_kind = named_item.value.syntax().kind();
            match item_kind {
                SyntaxKind::FUN => {
                    let fun_item = named_item.map(|it| it.cast_into::<ast::Fun>().unwrap());
                    self.instantiate_path_for_fun(path.into(), fun_item)
                }
                // lambdas
                SyntaxKind::IDENT_PAT => {
                    let ident_pat = named_item.map(|it| it.cast_into::<ast::IdentPat>().unwrap());
                    let binding_ty = self.get_binding_type(ident_pat.value);
                    binding_ty
                        .and_then(|it| it.into_ty_callable())
                        .unwrap_or(TyCallable::fake(call_expr.args().len(), CallKind::Lambda))
                }
                _ => TyCallable::fake(call_expr.args().len(), CallKind::Fun),
            }
        } else {
            TyCallable::fake(call_expr.args().len(), CallKind::Fun)
        };
        callable_ty
    }

    pub fn instantiate_path_for_fun(
        &self,
        method_or_path: ast::MethodOrPath,
        fun: InFile<ast::Fun>,
    ) -> TyCallable {
        let ty = self.instantiate_path(method_or_path, fun.in_file_into());
        match ty {
            Ty::Callable(ty_callable) => ty_callable,
            _ => unreachable!("instantiate_path() returns TyCallable for FUN items"),
        }
    }

    pub fn instantiate_path(
        &self,
        method_or_path: ast::MethodOrPath,
        generic_item: InFile<ast::AnyGenericItem>,
    ) -> Ty {
        let mut path_ty = self.ty_lowering().lower_path(
            method_or_path,
            generic_item.clone().map(|it| it.syntax().to_owned()),
        );

        let ty_vars_subst = generic_item.ty_vars_subst();
        path_ty = path_ty.substitute(&ty_vars_subst);

        path_ty
    }

    pub fn ty_lowering(&self) -> TyLowering {
        TyLowering::new(self.db)
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

    pub fn var_resolver(&self) -> TyVarResolver {
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
        ty.fold_with(self.var_resolver())
    }

    pub fn fully_resolve_vars(&self, ty: Ty) -> Ty {
        ty.fold_with(FullTyVarResolver::new(&self, Fallback::Unknown))
    }

    pub fn fully_resolve_vars_fallback_to_origin<T: TypeFoldable<T>>(&self, ty: T) -> T {
        ty.fold_with(FullTyVarResolver::new(&self, Fallback::Origin))
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

    pub fn inc_ty_counter(&mut self) -> usize {
        self.ty_var_counter = self.ty_var_counter + 1;
        self.ty_var_counter
    }

    pub fn get_binding_type(&self, ident_pat: ast::IdentPat) -> Option<Ty> {
        self.pat_types.get(&ident_pat.into()).map(|it| it.to_owned())
    }
}
