use crate::types::fold::TypeFoldable;
use crate::types::ty::Ty;
use crate::types::unification::{TyVarResolver, UnificationTable};

pub struct InferenceCtx {
    pub unification_table: UnificationTable,
}

impl InferenceCtx {
    pub fn new() -> Self {
        InferenceCtx {
            unification_table: UnificationTable::new(),
        }
    }

    pub fn resolve_vars_if_possible(&self, ty: Ty) -> Ty {
        ty.deep_fold_with(TyVarResolver::new(&self.unification_table))
    }
}
