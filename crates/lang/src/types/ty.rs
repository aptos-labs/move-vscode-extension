pub(crate) mod adt;
pub(crate) mod reference;
pub(crate) mod tuple;
pub(crate) mod ty_var;
pub(crate) mod type_param;

use crate::db::HirDatabase;
use crate::types::fold::TypeFoldable;
use crate::types::render::TypeRenderer;
use crate::types::ty::adt::TyAdt;
use crate::types::ty::reference::TyReference;
use crate::types::ty::tuple::TyTuple;
use crate::types::ty::ty_var::TyVar;
use crate::types::ty::type_param::TyTypeParameter;
use base_db::SourceRootDatabase;
use std::fmt;
use std::fmt::Formatter;

pub trait TypeFolder: Clone {
    fn fold_ty(&self, ty: Ty) -> Ty;
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

    Reference(TyReference),
    Vector(Box<Ty>),
    Adt(TyAdt),
    Tuple(TyTuple),
}

impl Ty {
    pub fn fold_with(self, folder: impl TypeFolder) -> Ty {
        folder.fold_ty(self)
    }

    pub fn unwrap_refs(&self) -> Ty {
        match self {
            Ty::Reference(ty_ref) => ty_ref.referenced().unwrap_refs(),
            _ => self.to_owned(),
        }
    }

    pub fn render(&self, db: &dyn SourceRootDatabase) -> String {
        TypeRenderer::new(db).render(self)
    }
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
