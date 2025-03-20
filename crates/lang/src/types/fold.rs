use crate::types::inference::InferenceCtx;
use crate::types::ty::ty_var::{TyInfer, TyVar, TyVarKind};
use crate::types::ty::type_param::TyTypeParameter;
use crate::types::ty::Ty;

pub trait TypeFoldable<T> {
    fn deep_fold_with(self, folder: impl TypeFolder) -> T;
}

pub trait TypeFolder: Clone {
    fn fold_ty(&self, ty: Ty) -> Ty;
}

#[derive(Clone)]
pub struct TyVarResolver<'a> {
    ctx: &'a InferenceCtx<'a>,
}

impl<'a> TyVarResolver<'a> {
    pub fn new(ctx: &'a InferenceCtx<'a>) -> Self {
        TyVarResolver { ctx }
    }
}

impl TypeFolder for TyVarResolver<'_> {
    fn fold_ty(&self, t: Ty) -> Ty {
        match t {
            Ty::Infer(ty_infer) => self.ctx.resolve_ty_infer(ty_infer),
            _ => t.deep_fold_with(self.to_owned()),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Fallback {
    TyUnknown,
    Origin,
}

#[derive(Clone)]
pub struct FullTyVarResolver<'a> {
    ctx: &'a InferenceCtx<'a>,
    fallback: Fallback,
}

impl<'a> FullTyVarResolver<'a> {
    pub fn new(ctx: &'a InferenceCtx<'a>, fallback: Fallback) -> Self {
        FullTyVarResolver { ctx, fallback }
    }
}

impl TypeFolder for FullTyVarResolver<'_> {
    fn fold_ty(&self, t: Ty) -> Ty {
        match t {
            Ty::Infer(ty_infer) => {
                let resolved_ty = self.ctx.resolve_ty_infer(ty_infer);
                match resolved_ty {
                    Ty::Unknown => Ty::Unknown,
                    Ty::Infer(ty_var) => match (self.fallback, &ty_var) {
                        (
                            Fallback::Origin,
                            TyInfer::Var(TyVar {
                                kind: TyVarKind::WithOrigin { origin_loc },
                            }),
                        ) => Ty::TypeParam(TyTypeParameter::from_loc(origin_loc.to_owned())),
                        _ => Ty::Unknown,
                    },
                    _ => resolved_ty,
                }
            }
            _ => t.deep_fold_with(self.to_owned()),
        }
    }
}
