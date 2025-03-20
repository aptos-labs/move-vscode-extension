pub(crate) mod ast_walker;
pub(crate) mod inference_result;

use crate::db::HirDatabase;
use crate::loc::SyntaxLocExt;
use crate::types::fold::{Fallback, FullTyVarResolver, TyVarResolver, TyVarVisitor};
use crate::types::has_type_params_ext::HasTypeParamsExt;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::lowering::TyLowering;
use crate::types::substitution::ApplySubstitution;
use crate::types::ty::adt::TyAdt;
use crate::types::ty::reference::TyReference;
use crate::types::ty::tuple::TyTuple;
use crate::types::ty::ty_var::{TyInfer, TyIntVar, TyVar};
use crate::types::ty::Ty;
use crate::types::unification::UnificationTable;
use crate::InFile;
use std::collections::HashMap;
use std::iter::zip;
use std::ops::Deref;
use syntax::{ast, AstNode};
use vfs::FileId;

pub struct InferenceCtx<'db> {
    pub db: &'db dyn HirDatabase,
    pub file_id: FileId,
    pub ty_var_counter: usize,

    pub var_table: UnificationTable<TyVar>,
    pub int_table: UnificationTable<TyIntVar>,

    pub expr_types: HashMap<ast::Expr, Ty>,
    pub pat_types: HashMap<ast::Pat, Ty>,
}

impl<'a> InferenceCtx<'a> {
    pub fn new(db: &'a dyn HirDatabase, file_id: FileId) -> Self {
        InferenceCtx {
            db,
            file_id,
            ty_var_counter: 0,
            var_table: UnificationTable::new(),
            int_table: UnificationTable::new(),
            expr_types: HashMap::new(),
            pat_types: HashMap::new(),
        }
    }

    pub fn infer(&mut self, ctx_owner: InFile<ast::InferenceCtxOwner>) {
        let InFile {
            file_id,
            value: ctx_owner,
        } = ctx_owner;

        {
            let mut ast_walker = TypeAstWalker::new(self);

            ast_walker.collect_parameter_bindings(&ctx_owner);
            match ctx_owner {
                ast::InferenceCtxOwner::Fun(fun) => {
                    if let Some(fun_block_expr) = fun.body() {
                        ast_walker.infer_block_expr(fun_block_expr);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn instantiate_path(&self, path: ast::Path, generic_item: InFile<ast::AnyGenericItem>) -> Ty {
        let ty_lowering = TyLowering::new(self.db, generic_item.file_id);
        let mut path_ty =
            ty_lowering.lower_path(path, generic_item.clone().map(|it| it.syntax().to_owned()));

        let ty_vars_subst = generic_item.ty_vars_subst();
        path_ty = path_ty.substitute(ty_vars_subst);

        path_ty
    }

    pub fn ty_lowering(&self) -> TyLowering {
        TyLowering::new(self.db, self.file_id)
    }

    pub fn resolve_ty_infer(&self, ty_infer: TyInfer) -> Ty {
        match &ty_infer {
            TyInfer::IntVar(ty_int_var) => self
                .int_table
                .resolve_to_ty_value(&ty_int_var)
                .unwrap_or(Ty::Infer(ty_infer)),
            TyInfer::Var(ty_var) => {
                let var_value_ty = self.var_table.resolve_to_ty_value(ty_var);
                match &var_value_ty {
                    None => Ty::Infer(ty_infer),
                    Some(Ty::Infer(TyInfer::IntVar(int_var))) => self
                        .int_table
                        .resolve_to_ty_value(int_var)
                        .unwrap_or(var_value_ty.unwrap()),
                    Some(_) => var_value_ty.unwrap(),
                }
            }
        }
    }

    pub fn resolve_vars_if_possible(&self, ty: Ty) -> Ty {
        ty.fold_with(TyVarResolver::new(&self))
    }

    pub fn fully_resolve_vars(&self, ty: Ty) -> Ty {
        ty.fold_with(FullTyVarResolver::new(&self, Fallback::TyUnknown))
    }

    pub fn fully_resolve_vars_fallback_to_origin(&self, ty: Ty) -> Ty {
        ty.fold_with(FullTyVarResolver::new(&self, Fallback::Origin))
    }

    fn resolve_ty_infer_shallow(&self, ty: Ty) -> Ty {
        if let Ty::Infer(ty_infer) = ty {
            self.resolve_ty_infer(ty_infer)
        } else {
            ty
        }
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
        let visitor = TyVarVisitor::new(|inner_ty_var| {
            &self.var_table.resolve_to_root_var(inner_ty_var) == ty_var
        });
        ty.visit_with(visitor)
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
            for ty in [left_ty, right_ty] {
                let ty_vars = ty.collect_ty_vars();
                for ty_var in ty_vars {
                    let _ = self.unify_ty_var(&ty_var, Ty::Unknown);
                }
            }
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
            // todo:
            _ => Ok(()),
        }
    }

    fn combine_ty_refs(&mut self, from_ref: TyReference, to_ref: TyReference) -> CombineResult {
        let is_mut_compat = from_ref.is_mut() || !to_ref.is_mut();
        if !is_mut_compat {
            return Err(TypeError::new(Ty::Reference(from_ref), Ty::Reference(to_ref)));
        }
        self.combine_types(from_ref.referenced().to_owned(), to_ref.referenced().to_owned())
    }

    fn combine_ty_adts(&mut self, ty1: TyAdt, ty2: TyAdt) -> CombineResult {
        Ok(())
    }

    fn combine_ty_tuples(&mut self, ty1: TyTuple, ty2: TyTuple) -> CombineResult {
        if ty1.types.len() != ty2.types.len() {
            return Err(TypeError::new(Ty::Tuple(ty1), Ty::Tuple(ty2)));
        }
        let ty_pairs = zip(ty1.types.into_iter(), ty2.types.into_iter()).collect();
        self.combine_ty_pairs(ty_pairs)
    }

    fn combine_ty_pairs(&mut self, ty_pairs: Vec<(Ty, Ty)>) -> CombineResult {
        let mut can_unify = Ok(());
        for (ty1, ty2) in ty_pairs {
            can_unify = can_unify.and(self.combine_types(ty1, ty2));
        }
        can_unify
    }
}

type CombineResult = Result<(), TypeError>;
struct TypeError {
    ty1: Ty,
    ty2: Ty,
}
impl TypeError {
    pub fn new(ty1: Ty, ty2: Ty) -> Self {
        TypeError { ty1, ty2 }
    }
}
