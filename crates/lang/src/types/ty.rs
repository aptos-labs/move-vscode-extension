pub(crate) mod adt;
pub(crate) mod reference;
pub(crate) mod tuple;
pub(crate) mod ty_var;
pub(crate) mod type_param;

use crate::db::HirDatabase;
use crate::types::fold::{TypeFoldable, TypeFolder, TypeVisitor};
use crate::types::render::TypeRenderer;
use crate::types::ty::adt::TyAdt;
use crate::types::ty::reference::TyReference;
use crate::types::ty::tuple::TyTuple;
use crate::types::ty::ty_var::TyInfer;
use crate::types::ty::type_param::TyTypeParameter;
use base_db::SourceRootDatabase;
use std::fmt;
use std::fmt::Formatter;

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

    Infer(TyInfer),
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

    pub fn visit_with(&self, visitor: impl TypeVisitor) -> bool {
        visitor.visit_ty(self)
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
            Ty::Reference(ty_ref) => Ty::Reference(TyReference::new(
                folder.fold_ty(ty_ref.referenced().to_owned()),
                ty_ref.mutability,
            )),
            _ => self,
        }
    }

    fn deep_visit_with(&self, visitor: impl TypeVisitor) -> bool {
        match self {
            Ty::Adt(ty_adt) => ty_adt.deep_visit_with(visitor),

            Ty::Vector(ty) => visitor.visit_ty(ty.as_ref()),
            Ty::Reference(ty_ref) => visitor.visit_ty(ty_ref.referenced()),

            _ => false,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum IntegerKind {
    Integer,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
}

impl IntegerKind {
    pub fn from_literal(lit: &str) -> Self {
        let lit = lit.to_lowercase();
        match lit {
            _ if lit.ends_with("u8") => IntegerKind::U8,
            _ if lit.ends_with("u16") => IntegerKind::U16,
            _ if lit.ends_with("u32") => IntegerKind::U32,
            _ if lit.ends_with("u64") => IntegerKind::U64,
            _ if lit.ends_with("u128") => IntegerKind::U128,
            _ if lit.ends_with("u256") => IntegerKind::U256,
            _ => IntegerKind::Integer
        }
    }

    pub fn is_default(&self) -> bool {
        *self == IntegerKind::Integer
    }
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
