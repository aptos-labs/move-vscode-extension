use crate::{AsName, Name};
use std::fmt;
use std::fmt::Formatter;
use syntax::ast;
use syntax::ast::HasName;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TyVar(TyVarKind);

impl TyVar {
    pub fn new_anonymous(index: u32) -> Self {
        TyVar(TyVarKind::Anonymous(index))
    }

    pub fn new_with_origin(origin: ast::TypeParam) -> Self {
        TyVar(TyVarKind::WithOrigin(origin))
    }
}

impl fmt::Display for TyVar {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.0 {
            TyVarKind::Anonymous(index) => write!(f, "?_{}", *index),
            TyVarKind::WithOrigin(origin) => {
                let origin_name = origin.name().map(|it| it.as_name());
                write!(f, "?_{}", origin_name.unwrap_or(Name::new("<anonymous>")))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyVarKind {
    Anonymous(u32),
    WithOrigin(ast::TypeParam),
}
