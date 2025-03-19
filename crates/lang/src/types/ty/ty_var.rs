use crate::loc::{SyntaxLoc, SyntaxLocExt};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TyVar {
    pub kind: TyVarKind,
}

impl TyVar {
    pub fn new_anonymous(index: usize) -> Self {
        TyVar {
            kind: TyVarKind::Anonymous(index),
        }
    }

    pub fn new_with_origin(origin_loc: SyntaxLoc) -> Self {
        TyVar {
            kind: TyVarKind::WithOrigin { origin_loc },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyVarKind {
    Anonymous(usize),
    WithOrigin { origin_loc: SyntaxLoc },
}
