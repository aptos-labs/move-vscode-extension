pub(crate) mod ast_walker;
pub(crate) mod inference_result;

use crate::db::HirDatabase;
use crate::loc::SyntaxLocExt;
use crate::types::fold::{Fallback, FullTyVarResolver, TyVarResolver};
use crate::types::has_type_params_ext::HasTypeParamsExt;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::inference::inference_result::InferenceResult;
use crate::types::lowering::TyLowering;
use crate::types::substitution::ApplySubstitution;
use crate::types::ty::ty_var::{TyInfer, TyIntVar, TyVar};
use crate::types::ty::Ty;
use crate::types::unification::UnificationTable;
use crate::InFile;
use std::collections::HashMap;
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

    pub fn infer(mut self, ctx_owner: InFile<ast::InferenceCtxOwner>) -> InferenceResult {
        let InFile {
            file_id,
            value: ctx_owner,
        } = ctx_owner;

        {
            let mut ast_walker = TypeAstWalker::new(&mut self);

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

        self.into_result(file_id)
    }

    fn into_result(self, file_id: FileId) -> InferenceResult {
        let expr_types = self
            .expr_types
            .clone()
            .into_iter()
            .map(|(expr, ty)| {
                let res_ty = self.fully_resolve_vars(ty);
                let expr_loc = InFile::new(file_id, expr).loc();
                (expr_loc, res_ty)
            })
            .collect();
        InferenceResult { file_id, expr_types }
    }

    pub fn instantiate_path(&self, path: ast::Path, generic_item: InFile<ast::AnyHasTypeParams>) -> Ty {
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
            (Ty::Infer(TyInfer::Var(ty_var)), right_ty) => self.unify_ty_var(ty_var, right_ty),
            (left_ty, Ty::Infer(TyInfer::Var(ty_var))) => self.unify_ty_var(ty_var, left_ty),

            (Ty::Infer(TyInfer::IntVar(int_var)), right_ty) => self.combine_int_var(int_var, right_ty),
            (left_ty, Ty::Infer(TyInfer::IntVar(int_var))) => self.combine_int_var(int_var, left_ty),

            (left_ty, right_ty) => self.combine_no_vars(left_ty, right_ty),
        }
    }

    fn unify_ty_var(&mut self, var: TyVar, ty: Ty) -> CombineResult {
        match ty {
            Ty::Infer(TyInfer::Var(ty_var)) => self.var_table.unify_var_var(&var, &ty_var),
            _ => {
                let root_ty_var = self.var_table.resolve_to_root_var(&var);
                // todo: cyclic type error
                self.var_table.unify_var_value(&root_ty_var, ty);
            }
        };
        Ok(())
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
            _ => {
                // todo: error
            }
        }
        Ok(())
    }

    fn combine_no_vars(&self, left_ty: Ty, right_ty: Ty) -> CombineResult {
        Ok(())
    }
}

type CombineResult = Result<(), TypeMismatchError>;
struct TypeMismatchError {
    ty1: Ty,
    ty2: Ty,
}
