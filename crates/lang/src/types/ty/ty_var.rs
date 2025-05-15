use crate::loc::SyntaxLoc;
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

    pub fn new_with_origin(origin_loc: SyntaxLoc, index: usize) -> Self {
        TyVar {
            kind: TyVarKind::WithOrigin { origin_loc, index },
        }
    }
}

impl fmt::Debug for TyVar {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.kind {
            TyVarKind::Anonymous(indx) => f.write_str(&format!("?_{}", indx)),
            TyVarKind::WithOrigin { origin_loc, index } => f.write_str(&format!(
                "?{}_{}",
                origin_loc.node_name().unwrap_or("".to_string()),
                index
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyVarKind {
    Anonymous(usize),
    WithOrigin { origin_loc: SyntaxLoc, index: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TyIntVar(usize);

impl TyIntVar {
    pub fn new(index: usize) -> Self {
        TyIntVar(index)
    }
}
