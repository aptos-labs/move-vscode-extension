use crate::types::inference::InferenceCtx;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::patterns::BindingMode::{BindByReference, BindByValue};
use crate::types::ty::Ty;
use crate::types::ty::reference::{Mutability, TyReference};
use crate::types::ty::tuple::TyTuple;
use syntax::ast;

impl TypeAstWalker<'_, '_> {
    pub fn collect_pat_bindings(&mut self, pat: ast::Pat, explicit_ty: Ty, def_bm: BindingMode) {
        match pat {
            ast::Pat::IdentPat(ident_pat) => {
                let ident_pat_ty = apply_bm(explicit_ty, def_bm);
                self.ctx.pat_types.insert(ident_pat.clone().into(), ident_pat_ty);
            }
            ast::Pat::StructPat(struct_pat) => {
                let (expected, _pat_bm) = strip_references(explicit_ty, def_bm);
                self.ctx
                    .pat_types
                    .insert(struct_pat.clone().into(), expected.clone());

                // let mut named_element = type_walker
                //     .ctx
                //     .resolve_path_cached(struct_pat.path(), Some(expected.clone()))
                //     .and_then(|item| item.cast::<ast::AnyFieldsOwner>());
                // if named_element.is_none() {
                //     named_element = expected.into_ty_adt().and_then(|it| {
                //         it.adt_item
                //             .cast_into::<ast::Struct>(type_walker.ctx.db.upcast())
                //             .into()
                //     });
                // }
            }
            _ => {}
        }
    }
}

pub fn anonymous_pat_ty_var(ctx: &mut InferenceCtx, pat: &ast::Pat) -> Ty {
    match pat {
        ast::Pat::IdentPat(_) => Ty::new_ty_var(ctx),
        ast::Pat::TuplePat(tuple_pat) => {
            let pat_types = tuple_pat.pats().map(|_| Ty::new_ty_var(ctx)).collect();
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
