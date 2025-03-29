use crate::types::substitution::Substitution;
use crate::types::ty::Ty;
use crate::types::ty::type_param::TyTypeParameter;
use syntax::ast;
use syntax::files::InFile;

pub trait GenericItemExt {
    fn ty_type_params(&self) -> Vec<TyTypeParameter>;
    fn ty_type_params_subst(&self) -> Substitution;
    fn ty_vars_subst(&self) -> Substitution;
}

impl<T: ast::GenericElement> GenericItemExt for InFile<T> {
    fn ty_type_params(&self) -> Vec<TyTypeParameter> {
        self.value
            .type_params()
            .into_iter()
            .map(|it| TyTypeParameter::new(InFile::new(self.file_id, it)))
            .collect()
    }

    fn ty_type_params_subst(&self) -> Substitution {
        let subst = self
            .ty_type_params()
            .into_iter()
            .map(|ty_tp| (ty_tp.clone(), Ty::TypeParam(ty_tp)))
            .collect();
        Substitution::new(subst)
    }

    fn ty_vars_subst(&self) -> Substitution {
        let subst = self
            .ty_type_params()
            .into_iter()
            .map(|ty_tp| (ty_tp.clone(), Ty::new_ty_var_with_origin(ty_tp.origin_loc)))
            .collect();
        Substitution::new(subst)
    }
}
