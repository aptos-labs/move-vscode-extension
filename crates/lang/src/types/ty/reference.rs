use crate::types::fold::{TypeFoldable, TypeFolder, TypeVisitor};
use crate::types::inference::InferenceCtx;
use crate::types::ty::Ty;
use std::ops::Deref;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TyReference {
    referenced: Box<Ty>,
    pub mutability: Mutability,
}

impl From<TyReference> for Ty {
    fn from(value: TyReference) -> Self {
        Ty::Reference(value)
    }
}

impl TyReference {
    pub fn new(inner: Ty, mutability: Mutability) -> Self {
        TyReference {
            referenced: Box::new(inner),
            mutability,
        }
    }

    pub fn referenced(&self) -> Ty {
        self.referenced.deref().to_owned()
    }

    pub fn is_mut(&self) -> bool {
        self.mutability.is_mut()
    }
}

impl TypeFoldable<TyReference> for TyReference {
    fn deep_fold_with(self, folder: impl TypeFolder) -> TyReference {
        TyReference::new(folder.fold_ty(self.referenced()), self.mutability)
    }

    fn deep_visit_with(&self, visitor: impl TypeVisitor) -> bool {
        visitor.visit_ty(&self.referenced())
    }
}

impl InferenceCtx<'_> {
    #[allow(clippy::wrong_self_convention)]
    pub fn is_tys_compatible_with_autoborrow(&mut self, ty: Ty, into_ty: Ty) -> bool {
        let Some(ty) = autoborrow(ty, &into_ty) else {
            return false;
        };
        self.is_tys_compatible(ty, into_ty)
    }
}

pub fn autoborrow(ty: Ty, into_ty: &Ty) -> Option<Ty> {
    match into_ty {
        Ty::Reference(into_ty_ref) => {
            match ty {
                Ty::Reference(ref ty_ref) => {
                    match (ty_ref.mutability, into_ty_ref.mutability) {
                        (Mutability::Mutable, Mutability::Mutable) => {
                            Some(Ty::Reference(ty_ref.to_owned()))
                        }
                        // & -> &mut (invalid)
                        (Mutability::Immutable, Mutability::Mutable) => None,
                        // &mut -> &
                        (Mutability::Mutable, Mutability::Immutable) => {
                            Some(Ty::new_reference(ty_ref.referenced(), Mutability::Immutable))
                        }
                        (Mutability::Immutable, Mutability::Immutable) => {
                            Some(Ty::Reference(ty_ref.to_owned()))
                        }
                    }
                }
                _ => Some(Ty::new_reference(ty, into_ty_ref.mutability)),
            }
        }
        _ => Some(ty),
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Mutability {
    Mutable,
    Immutable,
}

impl Mutability {
    pub fn new(is_mut: bool) -> Self {
        if is_mut {
            Mutability::Mutable
        } else {
            Mutability::Immutable
        }
    }

    pub fn is_mut(&self) -> bool {
        matches!(self, Mutability::Mutable)
    }

    pub fn intersect(&self, other: Mutability) -> Mutability {
        Self::new(self.is_mut() && other.is_mut())
    }
}
