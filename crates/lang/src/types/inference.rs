pub(crate) mod ast_walker;
pub(crate) mod combine_types;
pub(crate) mod inference_result;

use crate::db::HirDatabase;
use crate::nameres::binding::resolve_ident_pat_with_expected_type;
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
use std::collections::HashMap;
use std::hash::Hash;
use syntax::SyntaxKind::*;
use syntax::ast::FieldsOwner;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};
use vfs::FileId;

#[derive(Debug)]
pub struct InferenceCtx<'db> {
    pub db: &'db dyn HirDatabase,
    pub file_id: FileId,
    pub ty_var_counter: usize,
    pub msl: bool,

    pub var_table: UnificationTable<TyVar>,
    pub int_table: UnificationTable<TyIntVar>,

    pub pat_types: HashMap<ast::Pat, Ty>,
    pub pat_field_types: HashMap<ast::StructPatField, Ty>,
    pub expr_types: HashMap<ast::Expr, Ty>,
    pub expected_expr_types: HashMap<ast::Expr, Ty>,

    pub resolved_paths: HashMap<ast::Path, Vec<ScopeEntry>>,
    pub resolved_method_calls: HashMap<ast::MethodCallExpr, Option<ScopeEntry>>,
    pub resolved_fields: HashMap<ast::FieldRef, Option<ScopeEntry>>,
    pub resolved_ident_pats: HashMap<ast::IdentPat, Option<ScopeEntry>>,

    pub lambda_exprs: Vec<ast::LambdaExpr>,
    pub lambda_expr_types: HashMap<ast::LambdaExpr, TyCallable>,
}

impl<'db> InferenceCtx<'db> {
    pub fn new(db: &'db dyn HirDatabase, file_id: FileId, msl: bool) -> Self {
        InferenceCtx {
            db,
            file_id,
            ty_var_counter: 0,
            msl,
            var_table: UnificationTable::new(),
            int_table: UnificationTable::new(),
            expr_types: HashMap::new(),
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

    #[tracing::instrument(level = "debug", skip(self, path, expected_ty), fields(ctx_file_id = ?self.file_id))]
    pub fn resolve_path_cached(
        &mut self,
        path: ast::Path,
        expected_ty: Option<Ty>,
    ) -> Option<InFile<ast::AnyNamedElement>> {
        let entries =
            path_resolution::resolve_path(self.db, path.clone().in_file(self.file_id), expected_ty)
                .into_iter()
                .filter(|entry| {
                    // filter out bindings which are resolvable to enum variants
                    if let Some(ident_pat) = entry.clone().cast_into::<ast::IdentPat>(self.db) {
                        let res = self
                            .resolved_ident_pats
                            .get(&ident_pat.value)
                            .and_then(|it| it.clone());
                        if res.map(|it| it.node_loc.kind()) == Some(VARIANT) {
                            return false;
                        }
                    };
                    true
                })
                .collect::<Vec<_>>();

        self.resolved_paths.insert(path, entries.clone());

        entries
            .single_or_none()
            .and_then(|it| it.cast_into::<ast::AnyNamedElement>(self.db))
    }

    pub fn resolve_ident_pat_cached(
        &mut self,
        ident_pat: ast::IdentPat,
        expected_type: Option<Ty>,
    ) -> Option<InFile<ast::AnyNamedElement>> {
        let entry = resolve_ident_pat_with_expected_type(
            self.db,
            ident_pat.clone().in_file(self.file_id),
            expected_type,
        );
        self.resolved_ident_pats.insert(ident_pat, entry.clone());

        entry.and_then(|it| it.cast_into::<ast::AnyNamedElement>(self.db))
    }

    fn instantiate_call_expr_path(&mut self, call_expr: &ast::CallExpr) -> Option<TyCallable> {
        let path = call_expr.path();
        let named_item = self.resolve_path_cached(path.clone(), None);
        let callable_ty = if let Some(named_item) = named_item {
            let item_kind = named_item.value.syntax().kind();
            match item_kind {
                FUN | SPEC_FUN | SPEC_INLINE_FUN => {
                    let fun = named_item.cast_into::<ast::AnyFun>().unwrap();
                    self.instantiate_path_for_fun(path.into(), fun)
                }
                STRUCT | VARIANT => {
                    let fields_owner = named_item.cast_into::<ast::AnyFieldsOwner>().unwrap();
                    let call_ty = self.instantiate_call_expr_for_tuple_fields(path, fields_owner)?;
                    call_ty
                }
                // lambdas
                IDENT_PAT => {
                    let ident_pat = named_item.cast_into::<ast::IdentPat>().unwrap();
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
        Some(callable_ty)
    }

    fn instantiate_call_expr_for_tuple_fields(
        &mut self,
        path: ast::Path,
        fields_owner: InFile<ast::AnyFieldsOwner>,
    ) -> Option<TyCallable> {
        let (owner_file_id, fields_owner) = fields_owner.unpack();

        let owner_kind = fields_owner.syntax().kind();
        let (path, struct_or_enum): (ast::Path, ast::StructOrEnum) = match owner_kind {
            STRUCT => (path, fields_owner.struct_or_enum()),
            VARIANT => {
                let qualifier_path = path.qualifier()?;
                let variant = fields_owner.cast_into::<ast::Variant>()?;
                (qualifier_path, variant.enum_().into())
            }
            _ => unreachable!(),
        };
        let struct_or_enum = struct_or_enum.in_file(owner_file_id);

        let tuple_fields = fields_owner.tuple_field_list()?.fields().collect::<Vec<_>>();
        let param_types = tuple_fields
            .into_iter()
            .map(|it| {
                self.ty_lowering()
                    .lower_tuple_field(it.in_file(owner_file_id))
                    .unwrap_or(Ty::Unknown)
            })
            .collect::<Vec<_>>();
        let ret_type = Ty::new_ty_adt(struct_or_enum.clone());

        let ty_vars_subst = struct_or_enum.ty_vars_subst();
        let ctx_file_id = self.file_id;
        let type_args_subst = self
            .ty_lowering()
            .type_args_substitution(path.in_file(ctx_file_id).map_into(), struct_or_enum.map_into());

        let tuple_ty =
            TyCallable::new(param_types, ret_type, CallKind::Fun).substitute(&type_args_subst);

        Some(tuple_ty.substitute(&ty_vars_subst))
    }

    pub fn instantiate_path_for_fun(
        &mut self,
        method_or_path: ast::MethodOrPath,
        any_fun: InFile<ast::AnyFun>,
    ) -> TyCallable {
        let ty = self.instantiate_path(method_or_path, any_fun.map_into());
        match ty {
            Ty::Callable(ty_callable) => ty_callable,
            _ => unreachable!(
                "instantiate_path_for_fun() should return Ty::Callable, but returned {:?}",
                ty
            ),
        }
    }

    pub fn instantiate_path(
        &mut self,
        method_or_path: ast::MethodOrPath,
        generic_item: InFile<ast::AnyGenericElement>,
    ) -> Ty {
        let ctx_file_id = self.file_id;
        let ty_vars_subst = generic_item.ty_vars_subst();

        let mut path_ty = self
            .ty_lowering()
            .lower_path(method_or_path.in_file(ctx_file_id), generic_item.map_into());
        path_ty = path_ty.substitute(&ty_vars_subst);

        path_ty
    }

    pub fn ty_lowering(&mut self) -> TyLowering<'db> {
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
        if let Some(pat_field) = ident_pat.syntax().parent_of_type::<ast::StructPatField>() {
            return self.pat_field_types.get(&pat_field).map(|it| it.to_owned());
        }
        self.pat_types.get(&ident_pat.into()).cloned()
    }
}
