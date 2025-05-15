use crate::types::inference::InferenceCtx;
use crate::types::ty::Ty;
use crate::types::ty::ty_var::{TyInfer, TyVar, TyVarKind};
use crate::types::ty::type_param::TyTypeParameter;

pub trait TypeFoldable<T> {
    fn fold_with(self, folder: impl TypeFolder) -> T
    where
        Self: Sized,
    {
        self.deep_fold_with(folder)
    }

    fn visit_with(&self, visitor: impl TypeVisitor) -> bool {
        self.deep_visit_with(visitor)
    }

    fn deep_fold_with(self, folder: impl TypeFolder) -> T;
    fn deep_visit_with(&self, visitor: impl TypeVisitor) -> bool;
}

pub trait TypeFolder: Clone {
    fn fold_ty(&self, ty: Ty) -> Ty;

    fn fold_tys(&self, tys: Vec<Ty>) -> Vec<Ty> {
        tys.into_iter().map(|it| self.fold_ty(it)).collect()
    }

    fn fold_opt_ty(&self, ty: Option<Ty>) -> Option<Ty> {
        ty.map(|it| self.fold_ty(it))
    }
}

pub trait TypeVisitor: Clone {
    fn visit_ty(&self, ty: &Ty) -> bool;

    fn visit_tys(&self, tys: &Vec<Ty>) -> bool {
        tys.iter().fold(false, |acc, t| acc || self.visit_ty(t))
    }
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
            Ty::Infer(ty_infer) => self.ctx.resolve_ty_infer(&ty_infer),
            _ => t.deep_fold_with(self.to_owned()),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Fallback {
    Unknown,
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
                let resolved_ty = self.ctx.resolve_ty_infer(&ty_infer);
                match resolved_ty {
                    Ty::Infer(TyInfer::Var(ty_var)) => match (self.fallback, &ty_var) {
                        (
                            Fallback::Origin,
                            TyVar {
                                kind: TyVarKind::WithOrigin { origin_loc, .. },
                            },
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

#[derive(Clone)]
pub struct TyTypeParameterFolder<F: Fn(TyTypeParameter) -> Ty + Clone> {
    folder: F,
}

impl<F: Fn(TyTypeParameter) -> Ty + Clone> TyTypeParameterFolder<F> {
    pub fn new(folder: F) -> Self {
        TyTypeParameterFolder { folder }
    }
}

impl<F: Fn(TyTypeParameter) -> Ty + Clone> TypeFolder for TyTypeParameterFolder<F> {
    fn fold_ty(&self, ty: Ty) -> Ty {
        match ty {
            Ty::TypeParam(ty_type_param) => (self.folder)(ty_type_param),
            _ => ty.deep_fold_with(self.clone()),
        }
    }
}

impl Ty {
    pub fn fold_ty_type_params<F: Fn(TyTypeParameter) -> Ty + Clone>(self, folder: F) -> Ty
    where
        F: Sized,
    {
        let ty_type_param_folder = TyTypeParameterFolder::new(folder);
        ty_type_param_folder.fold_ty(self)
    }
}

#[derive(Clone)]
pub struct TyInferVisitor<V: Fn(&TyInfer) -> bool + Clone> {
    visitor: V,
}

impl<V: Fn(&TyInfer) -> bool + Clone> TyInferVisitor<V> {
    pub fn new(visitor: V) -> Self {
        TyInferVisitor { visitor }
    }
}

impl<V: Fn(&TyInfer) -> bool + Clone> TypeVisitor for TyInferVisitor<V> {
    fn visit_ty(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Infer(ty_infer) => (self.visitor)(ty_infer),
            _ => ty.deep_visit_with(self.clone()),
        }
    }
}

#[derive(Clone, Default)]
struct HasTyUnknownVisitor {}

impl TypeVisitor for HasTyUnknownVisitor {
    fn visit_ty(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Unknown => true,
            _ => ty.deep_visit_with(self.clone()),
        }
    }
}

impl Ty {
    pub fn deep_visit_ty_infers(&self, visitor: impl Fn(&TyInfer) -> bool + Clone) -> bool {
        let visitor = TyInferVisitor::new(visitor);
        self.visit_with(visitor)
    }

    pub fn has_ty_unknown(&self) -> bool {
        let visitor = HasTyUnknownVisitor::default();
        self.visit_with(visitor)
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, Ty::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ty::range_like::TySequence;
    use crate::types::ty::reference::{Mutability, TyReference};

    #[test]
    fn test_has_ty_unknown() {
        let ty = Ty::Reference(TyReference::new(
            Ty::Seq(TySequence::Vector(Box::new(Ty::Unknown))),
            Mutability::Immutable,
        ));
        assert!(ty.has_ty_unknown());
    }
}
