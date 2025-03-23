use crate::loc::{SyntaxLoc, SyntaxLocFileExt};
use std::fmt;
use std::fmt::Formatter;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyInfer {
    Var(TyVar),
    IntVar(TyIntVar),
}

#[derive(Clone, PartialEq, Eq, Hash)]
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

impl fmt::Debug for TyVar {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.kind {
            TyVarKind::Anonymous(indx) => f.write_str(&format!("?_{}", indx)),
            TyVarKind::WithOrigin { origin_loc } => f.write_str(&format!("?_({:?})", origin_loc)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyVarKind {
    Anonymous(usize),
    WithOrigin { origin_loc: SyntaxLoc },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TyIntVar(usize);

impl TyIntVar {
    pub fn new(index: usize) -> Self {
        TyIntVar(index)
    }
}
