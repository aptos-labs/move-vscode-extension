use crate::loc::{SyntaxLoc, SyntaxLocExt};
use crate::{AsName, Name};
use std::fmt;
use std::fmt::Formatter;
use syntax::ast::HasName;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TyVar {
    pub kind: TyVarKind,
}

impl TyVar {
    pub fn new_anonymous(index: u32) -> Self {
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
    Anonymous(u32),
    WithOrigin { origin_loc: SyntaxLoc },
}
