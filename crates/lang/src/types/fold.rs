use crate::types::inference::InferenceCtx;
use crate::types::ty::ty_var::{TyInfer, TyVar, TyVarKind};
use crate::types::ty::type_param::TyTypeParameter;
use crate::types::ty::Ty;
use std::cell::RefCell;

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
                let resolved_ty = self.ctx.resolve_ty_infer(&ty_infer);
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

#[derive(Clone)]
pub struct TyVarVisitor<V: Fn(&TyVar) -> bool + Clone> {
    visitor: V,
}

impl<V: Fn(&TyVar) -> bool + Clone> TyVarVisitor<V> {
    pub fn new(visitor: V) -> Self {
        TyVarVisitor { visitor }
    }
}

impl<V: Fn(&TyVar) -> bool + Clone> TypeVisitor for TyVarVisitor<V> {
    fn visit_ty(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Infer(TyInfer::Var(ty_var)) => (self.visitor)(ty_var),
            _ => ty.deep_visit_with(self.clone()),
        }
    }
}

impl Ty {
    pub fn collect_ty_vars(&self) -> Vec<TyVar> {
        let mut ty_vars = RefCell::new(vec![]);
        let collector = TyVarVisitor::new(|ty_var| {
            ty_vars.borrow_mut().push(ty_var.to_owned());
            false
        });
        self.visit_with(collector);
        ty_vars.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ty::reference::{Mutability, TyReference};

    #[test]
    fn test_ty_infer_visitor() {
        let ty_ref = Ty::Reference(TyReference::new(
            Ty::Infer(TyInfer::Var(TyVar::new_anonymous(0))),
            Mutability::Immutable,
        ));
        let res = ty_ref.collect_ty_vars();
        dbg!(res);
    }
}
