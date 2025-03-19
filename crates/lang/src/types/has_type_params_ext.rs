use crate::types::substitution::Substitution;
use crate::types::ty::ty_var::TyVar;
use crate::types::ty::type_param::TyTypeParameter;
use crate::types::ty::Ty;
use std::collections::HashMap;
use syntax::ast;
use syntax::ast::HasTypeParams;

pub trait HasTypeParamsExt {
    fn ty_type_params(&self) -> Vec<TyTypeParameter>;
    fn ty_type_params_subst(&self) -> Substitution;
    fn ty_vars_subst(&self) -> Substitution;
}

impl HasTypeParamsExt for ast::AnyHasTypeParams {
    fn ty_type_params(&self) -> Vec<TyTypeParameter> {
        self.type_params()
            .into_iter()
            .map(|it| TyTypeParameter::new(it))
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
            .map(|ty_tp| (ty_tp.clone(), Ty::Var(TyVar::new_with_origin(ty_tp.origin))))
            .collect();
        Substitution::new(subst)
    }
}
