pub(crate) mod adt;
pub(crate) mod type_param;

use crate::types::fold::TypeFoldable;
use crate::types::ty::adt::TyAdt;
use crate::types::ty::type_param::TyTypeParameter;
use crate::types::unification::TyVar;
use std::fmt;
use std::fmt::Formatter;

pub trait TypeFolder: Clone {
    fn fold_ty(&self, t: Ty) -> Ty;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Ty {
    Unknown,
    Never,

    Unit,
    Bool,
    Signer,
    Address,
    Integer(IntegerKind),
    Num,

    Var(TyVar),
    TypeParam(TyTypeParameter),

    Vector(Box<Ty>),
    Adt(TyAdt),
}

impl TypeFoldable<Ty> for Ty {
    fn deep_fold_with(self, folder: impl TypeFolder) -> Ty {
        match self {
            Ty::Adt(ty_adt) => Ty::Adt(ty_adt.deep_fold_with(folder)),
            Ty::Vector(ty) => Ty::Vector(Box::new(folder.fold_ty(*ty))),
            _ => self,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum IntegerKind {
    Integer,
    U8,
    U32,
    U64,
    U128,
    U256,
}

impl fmt::Display for IntegerKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            IntegerKind::Integer => "integer",
            _ => &format!("{:?}", self),
        };
        f.write_str(&s.to_lowercase())
    }
}
