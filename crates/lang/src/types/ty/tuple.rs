use crate::types::fold::{TypeFoldable, TypeFolder, TypeVisitor};
use crate::types::ty::Ty;
use crate::types::ty::adt::TyAdt;
use itertools::fold;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TyTuple {
    pub types: Vec<Ty>,
}

impl TyTuple {
    pub fn new(types: Vec<Ty>) -> Self {
        TyTuple { types }
    }
}

impl TypeFoldable<TyTuple> for TyTuple {
    fn deep_fold_with(self, folder: impl TypeFolder) -> TyTuple {
        TyTuple {
            types: folder.fold_tys(self.types),
        }
    }

    fn deep_visit_with(&self, visitor: impl TypeVisitor) -> bool {
        visitor.visit_tys(&self.types)
    }
}
