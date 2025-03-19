use crate::types::ty::Ty;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TyReference {
    referenced: Box<Ty>,
    pub mutability: Mutability,
}

impl TyReference {
    pub fn new(inner: Ty, mutability: Mutability) -> Self {
        TyReference {
            referenced: Box::new(inner),
            mutability,
        }
    }

    pub fn referenced(&self) -> &Ty {
        self.referenced.as_ref()
    }

    pub fn is_mut(&self) -> bool {
        self.mutability.is_mut()
    }
}

pub fn autoborrow(ty: Ty, into_ty: Ty) -> Option<Ty> {
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
                            Some(reference(ty_ref.referenced().to_owned(), Mutability::Immutable))
                        }
                        (Mutability::Immutable, Mutability::Immutable) => {
                            Some(Ty::Reference(ty_ref.to_owned()))
                        }
                    }
                }
                _ => Some(reference(ty, into_ty_ref.mutability)),
            }
        }
        _ => Some(ty),
    }
}

fn reference(ty: Ty, mutability: Mutability) -> Ty {
    Ty::Reference(TyReference::new(ty, mutability))
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
}
