use crate::InFile;
use crate::loc::{SyntaxLoc, SyntaxLocFileExt};
use syntax::ast;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TyTypeParameter {
    pub origin_loc: SyntaxLoc,
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
