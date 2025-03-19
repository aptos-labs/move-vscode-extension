pub(crate) mod ast_walker;
pub(crate) mod inference_result;

use crate::db::HirDatabase;
use crate::loc::SyntaxLocExt;
use crate::types::has_type_params_ext::HasTypeParamsExt;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::inference::inference_result::InferenceResult;
use crate::types::lowering::TyLowering;
use crate::types::substitution::ApplySubstitution;
use crate::types::ty::Ty;
use crate::types::unification::{Fallback, FullTyVarResolver, TyVarResolver, UnificationTable};
use crate::InFile;
use std::collections::HashMap;
use syntax::{ast, AstNode};
use vfs::FileId;

pub struct InferenceCtx<'db> {
    pub db: &'db dyn HirDatabase,
    pub var_unification_table: UnificationTable,

    pub expr_types: HashMap<ast::Expr, Ty>,
    pub pat_types: HashMap<ast::Pat, Ty>,
}

impl<'a> InferenceCtx<'a> {
    pub fn new(db: &'a dyn HirDatabase) -> Self {
        InferenceCtx {
            db,
            var_unification_table: UnificationTable::new(),
            expr_types: HashMap::new(),
            pat_types: HashMap::new(),
        }
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

    pub fn infer(mut self, ctx_owner: InFile<ast::InferenceCtxOwner>) -> InferenceResult {
        let InFile {
            file_id,
            value: ctx_owner,
        } = ctx_owner;

        {
            let mut ast_walker = TypeAstWalker::new(&mut self, file_id);

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

    pub fn instantiate_path(&self, path: ast::Path, generic_item: InFile<ast::AnyHasTypeParams>) -> Ty {
        let ty_lowering = TyLowering::new(self.db, generic_item.file_id);
        let mut path_ty =
            ty_lowering.lower_path(path, generic_item.clone().map(|it| it.syntax().to_owned()));

        let ty_vars_subst = generic_item.ty_vars_subst();
        path_ty = path_ty.substitute(ty_vars_subst);

        path_ty
    }

    pub fn resolve_vars_if_possible(&self, ty: Ty) -> Ty {
        ty.fold_with(TyVarResolver::new(&self.var_unification_table))
    }

    pub fn fully_resolve_vars(&self, ty: Ty) -> Ty {
        ty.fold_with(FullTyVarResolver::new(
            &self.var_unification_table,
            Fallback::TyUnknown,
        ))
    }

    pub fn fully_resolve_vars_fallback_to_origin(&self, ty: Ty) -> Ty {
        ty.fold_with(FullTyVarResolver::new(
            &self.var_unification_table,
            Fallback::Origin,
        ))
    }
}
