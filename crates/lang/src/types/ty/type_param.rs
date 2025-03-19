use crate::loc::{SyntaxLoc, SyntaxLocExt};
use crate::{loc, InFile};
use syntax::ast;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TyTypeParameter {
    pub origin_loc: loc::SyntaxLoc,
}

impl TyTypeParameter {
    pub fn new(origin: InFile<ast::TypeParam>) -> Self {
        TyTypeParameter {
            origin_loc: origin.loc(),
        }
    }

    pub fn from_loc(origin_loc: SyntaxLoc) -> Self {
        TyTypeParameter { origin_loc }
    }
}
