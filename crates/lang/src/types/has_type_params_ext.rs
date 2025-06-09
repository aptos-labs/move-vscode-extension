use crate::types::inference::TyVarIndex;
use crate::types::substitution::Substitution;
use crate::types::ty::Ty;
use crate::types::ty::type_param::TyTypeParameter;
use syntax::ast;
use syntax::files::InFile;

pub trait GenericItemExt {
    fn generic_element(&self) -> InFile<ast::GenericElement>;

    // fn ty_type_params(&self) -> Vec<TyTypeParameter> {
    //     self.value
    //         .type_params()
    //         .into_iter()
    //         .map(|it| TyTypeParameter::new(InFile::new(self.file_id, it)))
    //         .collect()
    // }

    fn ty_type_params(&self) -> Vec<TyTypeParameter> {
        let generic_item = self.generic_element();
        generic_item
            .value
            .type_params()
            .into_iter()
            .map(|it| TyTypeParameter::new(InFile::new(generic_item.file_id, it)))
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

    /// Substitution `TyTypeParam -> TyVar(origin=TypeParam)`.
    fn ty_vars_subst(&self, ty_var_index: &TyVarIndex) -> Substitution {
        let subst = self
            .ty_type_params()
            .into_iter()
            .map(|ty_tp| {
                (
                    ty_tp.clone(),
                    Ty::new_ty_var_with_origin(ty_tp.origin_loc, ty_var_index),
                )
            })
            .collect();
        Substitution::new(subst)
    }

    // fn ty_type_params_subst(&self) -> Substitution;
    // fn ty_vars_subst(&self, ty_var_index: &TyVarIndex) -> Substitution;
}

impl<T: Into<ast::GenericElement> + Clone> GenericItemExt for InFile<T> {
    fn generic_element(&self) -> InFile<ast::GenericElement> {
        self.clone().map(|it| it.into())
    }

    // fn ty_type_params(&self) -> Vec<TyTypeParameter> {
    //     self.value
    //         .type_params()
    //         .into_iter()
    //         .map(|it| TyTypeParameter::new(InFile::new(self.file_id, it)))
    //         .collect()
    // }
    //
    // fn ty_type_params_subst(&self) -> Substitution {
    //     let subst = self
    //         .ty_type_params()
    //         .into_iter()
    //         .map(|ty_tp| (ty_tp.clone(), Ty::TypeParam(ty_tp)))
    //         .collect();
    //     Substitution::new(subst)
    // }
    //
    // /// Substitution `TyTypeParam -> TyVar(origin=TypeParam)`.
    // fn ty_vars_subst(&self, ty_var_index: &TyVarIndex) -> Substitution {
    //     let subst = self
    //         .ty_type_params()
    //         .into_iter()
    //         .map(|ty_tp| {
    //             (
    //                 ty_tp.clone(),
    //                 Ty::new_ty_var_with_origin(ty_tp.origin_loc, ty_var_index),
    //             )
    //         })
    //         .collect();
    //     Substitution::new(subst)
    // }
}
