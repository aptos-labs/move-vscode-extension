use crate::db::HirDatabase;
use crate::types::fold::TypeFoldable;
use crate::types::has_type_params_ext::HasTypeParamsExt;
use crate::types::lowering::TyLowering;
use crate::types::substitution::ApplySubstitution;
use crate::types::ty::Ty;
use crate::types::unification::{TyVarResolver, UnificationTable};
use crate::InFile;
use syntax::{ast, AstNode};

pub struct InferenceCtx<'a> {
    pub db: &'a dyn HirDatabase,
    pub var_unification_table: UnificationTable,
}

impl<'a> InferenceCtx<'a> {
    pub fn new(db: &'a dyn HirDatabase) -> Self {
        InferenceCtx {
            db,
            var_unification_table: UnificationTable::new(),
        }
    }

    pub fn instantiate_path(&self, path: ast::Path, generic_item: InFile<ast::AnyHasTypeParams>) -> Ty {
        let ty_lowering = TyLowering::new(self.db, generic_item.file_id);
        let mut path_ty =
            ty_lowering.lower_path(path, generic_item.clone().map(|it| it.syntax().to_owned()));

        let ty_vars_subst = generic_item.value.ty_vars_subst();
        path_ty = path_ty.substitute(ty_vars_subst);

        path_ty
    }

    pub fn resolve_vars_if_possible(&self, ty: Ty) -> Ty {
        ty.deep_fold_with(TyVarResolver::new(&self.var_unification_table))
    }
}
