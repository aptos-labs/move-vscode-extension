use crate::loc::{SyntaxLoc, SyntaxLocExt};
use crate::types::fold::{TypeFoldable, TypeFolder};
use crate::types::substitution::{empty_substitution, Substitution};
use crate::InFile;
use syntax::ast;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TyAdt {
    item: SyntaxLoc,
    subst: Substitution,
}

impl TyAdt {
    pub fn new(item: InFile<ast::StructOrEnum>) -> Self {
        TyAdt {
            item: item.loc(),
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
