use crate::types::fold::TypeFoldable;
use crate::types::ty::type_param::TyTypeParameter;
use crate::types::ty::{Ty, TypeFolder};
use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Substitution {
    mapping: HashMap<TyTypeParameter, Ty>,
}

impl Substitution {
    pub fn new(mapping: HashMap<TyTypeParameter, Ty>) -> Self {
        Substitution { mapping }
    }

    pub fn get_ty(&self, ty_tp: &TyTypeParameter) -> Option<Ty> {
        self.mapping.get(ty_tp).map(|it| it.to_owned())
    }
}

pub fn empty_substitution() -> Substitution {
    Substitution::default()
}

impl TypeFoldable<Substitution> for Substitution {
    fn deep_fold_with(self, folder: impl TypeFolder) -> Substitution {
        let folded_mapping = self
            .mapping
            .into_iter()
            .map(|(k, v)| (k, v.deep_fold_with(folder.clone())))
            .collect();
        Substitution {
            mapping: folded_mapping,
        }
    }
}

pub trait ApplySubstitution {
    type Item: TypeFoldable<Self::Item>;

    fn substitute(self, subst: Substitution) -> Self::Item;
}

#[derive(Debug, Clone)]
pub struct SubstitutionApplier {
    subst: Substitution,
}

impl TypeFolder for SubstitutionApplier {
    fn fold_ty(&self, ty: Ty) -> Ty {
        match ty {
            Ty::TypeParam(ty_tp) => self.subst.get_ty(&ty_tp).unwrap_or(Ty::TypeParam(ty_tp)),
            _ => ty.deep_fold_with(self.to_owned()),
        }
    }
}

impl<T: TypeFoldable<T>> ApplySubstitution for T {
    type Item = T;

    fn substitute(self, subst: Substitution) -> T {
        self.deep_fold_with(SubstitutionApplier { subst })
    }
}
