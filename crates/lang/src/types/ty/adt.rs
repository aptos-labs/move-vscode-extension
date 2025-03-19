use crate::types::fold::TypeFoldable;
use crate::types::substitution::{empty_substitution, Substitution};
use crate::types::ty::TypeFolder;
use syntax::ast;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TyAdt {
    item: ast::StructOrEnum,
    subst: Substitution,
}

impl TyAdt {
    pub fn new(item: ast::StructOrEnum) -> Self {
        TyAdt {
            item,
            subst: empty_substitution(),
        }
    }
}

impl TypeFoldable<TyAdt> for TyAdt {
    fn deep_fold_with(self, folder: impl TypeFolder) -> TyAdt {
        TyAdt {
            item: self.item,
            subst: self.subst.deep_fold_with(folder),
        }
    }
}
