pub(crate) mod ast_walker;
pub(crate) mod inference_result;

use crate::db::HirDatabase;
use crate::files::{InFileExt, InFileInto};
use crate::nameres::path_resolution;
use crate::nameres::scope::{ScopeEntry, VecExt};
use crate::types::fold::{Fallback, FullTyVarResolver, TyVarResolver, TypeFoldable};
use crate::types::has_type_params_ext::GenericItemExt;
use crate::types::lowering::TyLowering;
use crate::types::substitution::ApplySubstitution;
use crate::types::ty::adt::TyAdt;
use crate::types::ty::reference::TyReference;
use crate::types::ty::tuple::TyTuple;
use crate::types::ty::ty_callable::TyCallable;
use crate::types::ty::ty_var::{TyInfer, TyIntVar, TyVar};
use crate::types::ty::Ty;
use crate::types::unification::UnificationTable;
use crate::InFile;
use parser::SyntaxKind;
use std::cell::RefCell;
use std::collections::HashMap;
use std::iter::zip;
use std::ops::Deref;
use syntax::{ast, AstNode, SyntaxNodeOrToken};
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
                _ => TyCallable::fake(call_expr.args().len()),
            }
        } else {
            TyCallable::fake(call_expr.args().len())
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
        let ty_lowering = TyLowering::new(self.db, generic_item.file_id);
        let mut path_ty = ty_lowering.lower_path(
            method_or_path,
            generic_item.clone().map(|it| it.syntax().to_owned()),
        );

        let ty_vars_subst = generic_item.ty_vars_subst();
        path_ty = path_ty.substitute(ty_vars_subst);

        path_ty
    }

    pub fn ty_lowering(&self) -> TyLowering {
        TyLowering::new(self.db, self.file_id)
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

    pub fn resolve_vars_if_possible(&self, ty: Ty) -> Ty {
        ty.fold_with(self.var_resolver())
    }

    pub fn fully_resolve_vars(&self, ty: Ty) -> Ty {
        ty.fold_with(FullTyVarResolver::new(&self, Fallback::Unknown))
    }

    pub fn fully_resolve_vars_fallback_to_origin(&self, ty: Ty) -> Ty {
        ty.fold_with(FullTyVarResolver::new(&self, Fallback::Origin))
    }

    fn resolve_ty_infer_shallow(&self, ty: Ty) -> Ty {
        if let Ty::Infer(ty_infer) = ty {
            self.resolve_ty_infer(&ty_infer)
        } else {
            ty
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn is_tys_compatible(&mut self, ty: Ty, into_ty: Ty) -> bool {
        self.freeze(|ctx| ctx.combine_types(ty, into_ty).is_ok())
    }

    pub fn combine_types(&mut self, left_ty: Ty, right_ty: Ty) -> CombineResult {
        let left_ty = self.resolve_ty_infer_shallow(left_ty);
        let right_ty = self.resolve_ty_infer_shallow(right_ty);

        match (left_ty, right_ty) {
            (Ty::Infer(TyInfer::Var(ty_var)), right_ty) => self.unify_ty_var(&ty_var, right_ty),
            (left_ty, Ty::Infer(TyInfer::Var(ty_var))) => self.unify_ty_var(&ty_var, left_ty),

            (Ty::Infer(TyInfer::IntVar(int_var)), right_ty) => self.combine_int_var(int_var, right_ty),
            (left_ty, Ty::Infer(TyInfer::IntVar(int_var))) => self.combine_int_var(int_var, left_ty),

            (left_ty, right_ty) => self.combine_no_vars(left_ty, right_ty),
        }
    }

    fn unify_ty_var(&mut self, var: &TyVar, ty: Ty) -> CombineResult {
        match ty {
            Ty::Infer(TyInfer::Var(ty_var)) => self.var_table.unify_var_var(var, &ty_var),
            _ => {
                let root_ty_var = self.var_table.resolve_to_root_var(var);
                if self.ty_contains_ty_var(&ty, &root_ty_var) {
                    // "E0308 cyclic type of infinite size"
                    self.var_table.unify_var_value(&root_ty_var, Ty::Unknown);
                    return Ok(());
                }
                self.var_table.unify_var_value(&root_ty_var, ty);
            }
        };
        Ok(())
    }

    fn ty_contains_ty_var(&self, ty: &Ty, ty_var: &TyVar) -> bool {
        // let visitor = TyInferVisitor::new(|inner_ty_var| {
        //     &self.var_table.resolve_to_root_var(inner_ty_var) == ty_var
        // });
        ty.deep_visit_ty_infers(|ty_infer| match ty_infer {
            TyInfer::Var(inner_ty_var) => &self.var_table.resolve_to_root_var(&inner_ty_var) == ty_var,
            _ => false,
        })
    }

    fn combine_int_var(&mut self, int_var: TyIntVar, ty: Ty) -> CombineResult {
        match ty {
            Ty::Infer(TyInfer::IntVar(ty_int_var)) => {
                self.int_table.unify_var_var(&int_var, &ty_int_var)
            }
            Ty::Integer(_) => self.int_table.unify_var_value(&int_var, ty),
            Ty::Unknown => {
                // do nothing, unknown should not influence IntVar
            }
            _ => return Err(TypeError::new(Ty::Infer(TyInfer::IntVar(int_var)), ty)),
        }
        Ok(())
    }

    fn combine_no_vars(&mut self, left_ty: Ty, right_ty: Ty) -> CombineResult {
        // assign Ty::Unknown to all inner `TyVar`s if other type is unknown
        if matches!(left_ty, Ty::Unknown) || matches!(right_ty, Ty::Unknown) {
            self.unify_ty_vars_with_unknown(vec![left_ty, right_ty]);
            return Ok(());
        }
        // if never type is involved, do not perform comparison
        if matches!(left_ty, Ty::Never) || matches!(right_ty, Ty::Never) {
            return Ok(());
        }
        // if type are exactly equal, then they're compatible
        if left_ty == right_ty {
            return Ok(());
        }

        match (&left_ty, &right_ty) {
            (Ty::Integer(kind1), Ty::Integer(kind2)) => {
                if kind1.is_default() || kind2.is_default() {
                    return Ok(());
                }
                Err(TypeError::new(left_ty, right_ty))
            }
            (Ty::Vector(ty1), Ty::Vector(ty2)) => {
                self.combine_types(ty1.deref().to_owned(), ty2.deref().to_owned())
            }
            (Ty::Reference(from_ref), Ty::Reference(to_ref)) => self.combine_ty_refs(from_ref, to_ref),
            (Ty::Callable(ty_call1), Ty::Callable(ty_call2)) => {
                self.combine_ty_callables(ty_call1, ty_call2)
            }

            (Ty::Adt(ty_adt1), Ty::Adt(ty_adt2)) => self.combine_ty_adts(ty_adt1, ty_adt2),
            (Ty::Tuple(ty_tuple1), Ty::Tuple(ty_tuple2)) => self.combine_ty_tuples(ty_tuple1, ty_tuple2),

            _ => Err(TypeError::new(left_ty, right_ty)),
        }
    }

    fn unify_ty_vars_with_unknown(&mut self, tys: Vec<Ty>) {
        let ty_vars = RefCell::new(vec![]);
        for ty in tys {
            ty.deep_visit_ty_infers(|ty_infer| {
                if let TyInfer::Var(ty_var) = ty_infer {
                    ty_vars.borrow_mut().push(ty_var.clone());
                };
                false
            });
        }
        for ty_var in ty_vars.into_inner() {
            let _ = self.unify_ty_var(&ty_var, Ty::Unknown);
        }
    }

    fn combine_ty_refs(&mut self, from_ref: &TyReference, to_ref: &TyReference) -> CombineResult {
        let is_mut_compat = from_ref.is_mut() || !to_ref.is_mut();
        if !is_mut_compat {
            return Err(TypeError::new(
                Ty::Reference(from_ref.to_owned()),
                Ty::Reference(to_ref.to_owned()),
            ));
        }
        self.combine_types(from_ref.referenced().to_owned(), to_ref.referenced().to_owned())
    }

    fn combine_ty_callables(&mut self, ty1: &TyCallable, ty2: &TyCallable) -> CombineResult {
        // todo: check param types size
        self.combine_ty_pairs(ty1.clone().param_types, ty2.clone().param_types)?;
        // todo: resolve variables?
        self.combine_types(ty1.ret_type.deref().to_owned(), ty2.ret_type.deref().to_owned())
    }

    fn combine_ty_adts(&mut self, ty1: &TyAdt, ty2: &TyAdt) -> CombineResult {
        if ty1.adt_item != ty2.adt_item {
            return Err(TypeError::new(Ty::Adt(ty1.to_owned()), Ty::Adt(ty2.to_owned())));
        }
        Ok(())
    }

    fn combine_ty_tuples(&mut self, ty1: &TyTuple, ty2: &TyTuple) -> CombineResult {
        if ty1.types.len() != ty2.types.len() {
            return Err(TypeError::new(
                Ty::Tuple(ty1.to_owned()),
                Ty::Tuple(ty2.to_owned()),
            ));
        }
        self.combine_ty_pairs(ty1.clone().types, ty2.clone().types)
    }

    fn combine_ty_pairs(&mut self, left_tys: Vec<Ty>, right_tys: Vec<Ty>) -> CombineResult {
        let mut can_unify = Ok(());
        let pairs = zip(left_tys.into_iter(), right_tys.into_iter());
        for (ty1, ty2) in pairs {
            can_unify = can_unify.and(self.combine_types(ty1, ty2));
        }
        can_unify
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

    pub fn coerce_types(&mut self, node_or_token: SyntaxNodeOrToken, actual: Ty, expected: Ty) -> bool {
        let actual = self.resolve_vars_if_possible(actual);
        let expected = self.resolve_vars_if_possible(expected);
        if actual == expected {
            return true;
        }
        let combined = self.combine_types(actual.clone(), expected.clone());
        match combined {
            Ok(()) => true,
            Err(type_error) => {
                // todo: report type error at `node`
                self.report_type_error(type_error, node_or_token, actual, expected);
                false
            }
        }
    }

    pub fn get_binding_type(&self, ident_pat: ast::IdentPat) -> Option<Ty> {
        self.pat_types.get(&ident_pat.into()).map(|it| it.to_owned())
    }

    fn report_type_error(
        &mut self,
        _type_error: TypeError,
        _node_or_token: SyntaxNodeOrToken,
        _actual: Ty,
        _expected: Ty,
    ) {
        // todo: report type error at `node`
    }
}

pub type CombineResult = Result<(), TypeError>;
pub struct TypeError {
    ty1: Ty,
    ty2: Ty,
}
impl TypeError {
    pub fn new(ty1: Ty, ty2: Ty) -> Self {
        TypeError { ty1, ty2 }
    }
}
