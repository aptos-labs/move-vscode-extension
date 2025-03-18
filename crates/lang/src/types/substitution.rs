use crate::types::fold::TypeFoldable;
use crate::types::ty::type_param::TyTypeParameter;
use crate::types::ty::{Ty, TypeFolder};
use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Substitution {
    mapping: HashMap<TyTypeParameter, Ty>,
}

impl TypeFoldable<Substitution> for Substitution {
    fn deep_fold_with(self, folder: impl TypeFolder) -> Substitution
    {
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
