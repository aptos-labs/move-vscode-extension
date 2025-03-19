use crate::types::patterns::BindingMode::{BindByReference, BindByValue};
use crate::types::ty::reference::{Mutability, TyReference};
use crate::types::ty::Ty;

#[derive(Debug, Clone)]
pub enum BindingMode {
    BindByValue,
    BindByReference { mutability: Mutability },
}

fn apply_bm(ty: Ty, def_bm: BindingMode) -> Ty {
    match def_bm {
        BindByReference { mutability } => Ty::Reference(TyReference::new(ty, mutability)),
        BindByValue => ty,
    }
}

fn strip_references(ty: Ty, def_bm: BindingMode) -> (Ty, BindingMode) {
    let mut bm = def_bm;
    let mut ty = ty;
    while let Ty::Reference(ty_ref) = &ty {
        bm = match bm.clone() {
            BindByReference { mutability: old_mut } => {
                let new_mutability = if old_mut == Mutability::Immutable {
                    Mutability::Immutable
                } else {
                    ty_ref.mutability.to_owned()
                };
                BindByReference {
                    mutability: new_mutability,
                }
            }
            BindByValue => BindByReference {
                mutability: ty_ref.mutability.to_owned(),
            },
        };
        ty = ty_ref.referenced().to_owned();
    }
    (ty, bm)
}
