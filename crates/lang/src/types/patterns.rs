use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::patterns::BindingMode::{BindByReference, BindByValue};
use crate::types::ty::reference::{Mutability, TyReference};
use crate::types::ty::tuple::TyTuple;
use crate::types::ty::ty_var::{TyInfer, TyVar};
use crate::types::ty::Ty;
use syntax::ast;

pub fn collect_bindings(
    ast_walker: &mut TypeAstWalker,
    pat: ast::Pat,
    explicit_ty: Ty,
    def_bm: BindingMode,
) {
    match pat {
        ast::Pat::IdentPat(ident_pat) => {
            let ident_pat_ty = apply_bm(explicit_ty, def_bm);
            ast_walker
                .ctx
                .pat_types
                .insert(ast::Pat::IdentPat(ident_pat), ident_pat_ty);
        }
        _ => {}
    }
}

pub fn anonymous_pat_ty_var(counter: &mut usize, pat: &ast::Pat) -> Ty {
    match pat {
        ast::Pat::IdentPat(_) => {
            *counter = *counter + 1;
            Ty::Infer(TyInfer::Var(TyVar::new_anonymous(*counter)))
        }
        ast::Pat::TuplePat(tuple_pat) => {
            let pat_types = tuple_pat
                .pats()
                .map(|pat| {
                    *counter = *counter + 1;
                    Ty::Infer(TyInfer::Var(TyVar::new_anonymous(*counter)))
                })
                .collect();
            Ty::Tuple(TyTuple::new(pat_types))
        }
        _ => Ty::Unknown,
    }
}

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
