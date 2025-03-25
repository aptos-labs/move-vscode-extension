pub mod adt;
pub(crate) mod integer;
pub mod reference;
pub(crate) mod tuple;
pub(crate) mod ty_callable;
pub(crate) mod ty_var;
pub(crate) mod type_param;

use crate::db::HirDatabase;
use crate::loc::SyntaxLoc;
use crate::types::fold::{TypeFoldable, TypeFolder, TypeVisitor};
use crate::types::render::TypeRenderer;
use crate::types::ty::adt::TyAdt;
use crate::types::ty::integer::IntegerKind;
use crate::types::ty::reference::TyReference;
use crate::types::ty::tuple::TyTuple;
use crate::types::ty::ty_callable::TyCallable;
use crate::types::ty::ty_var::{TyInfer, TyVar};
use crate::types::ty::type_param::TyTypeParameter;
use crate::InFile;
use base_db::SourceRootDatabase;
use std::fmt;
use std::fmt::Formatter;
use syntax::{ast, AstToken};

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

    Range(Box<Ty>),
    Vector(Box<Ty>),

    Infer(TyInfer),
    TypeParam(TyTypeParameter),

    Reference(TyReference),
    Adt(TyAdt),
    Callable(TyCallable),
    Tuple(TyTuple),
}

impl Ty {
    pub fn ty_var_with_origin(tp_origin_loc: SyntaxLoc) -> Ty {
        Ty::Infer(TyInfer::Var(TyVar::new_with_origin(tp_origin_loc)))
    }

    pub fn deref(&self) -> Ty {
        match self {
            Ty::Reference(ty_ref) => ty_ref.referenced().deref(),
            _ => self.to_owned(),
        }
    }

    pub fn item_module(&self, db: &dyn HirDatabase) -> Option<InFile<ast::Module>> {
        let ty = self.deref();
        match ty {
            Ty::Adt(ty_adt) => {
                let item = ty_adt
                    .adt_item
                    .cast_into::<ast::StructOrEnum>(db.upcast())
                    .unwrap();
                Some(item.map(|it| it.module()))
            }
            // todo: vector
            _ => None,
        }
    }

    pub fn into_ty_callable(self) -> Option<TyCallable> {
        match self {
            Ty::Callable(ty_callable) => Some(ty_callable),
            _ => None,
        }
    }

    pub fn into_ty_ref(self) -> Option<TyReference> {
        match self {
            Ty::Reference(ty_ref) => Some(ty_ref),
            _ => None,
        }
    }

    pub fn into_ty_adt(self) -> Option<TyAdt> {
        match self {
            Ty::Adt(ty_adt) => Some(ty_adt),
            _ => None,
        }
    }

    pub fn render(&self, db: &dyn SourceRootDatabase) -> String {
        TypeRenderer::new(db).render(self)
    }
}

impl TypeFoldable<Ty> for Ty {
    fn fold_with(self, folder: impl TypeFolder) -> Ty {
        folder.fold_ty(self)
    }

    fn visit_with(&self, visitor: impl TypeVisitor) -> bool {
        visitor.visit_ty(self)
    }

    fn deep_fold_with(self, folder: impl TypeFolder) -> Ty {
        match self {
            Ty::Adt(ty_adt) => Ty::Adt(ty_adt.deep_fold_with(folder)),
            Ty::Vector(ty) => Ty::Vector(Box::new(folder.fold_ty(*ty))),
            Ty::Reference(ty_ref) => Ty::Reference(TyReference::new(
                folder.fold_ty(ty_ref.referenced().to_owned()),
                ty_ref.mutability,
            )),
            Ty::Callable(ty_callable) => Ty::Callable(ty_callable.deep_fold_with(folder)),
            _ => self,
        }
    }

    fn deep_visit_with(&self, visitor: impl TypeVisitor) -> bool {
        match self {
            Ty::Adt(ty_adt) => ty_adt.deep_visit_with(visitor),
            Ty::Vector(ty) => visitor.visit_ty(ty.as_ref()),
            Ty::Reference(ty_ref) => visitor.visit_ty(ty_ref.referenced()),
            Ty::Callable(ty_callable) => ty_callable.deep_visit_with(visitor),
            _ => false,
        }
    }
}
