use crate::types::inference::InferenceCtx;
use crate::types::ty::Ty;

pub enum Expectation {
    NoValue,
    ExpectType(Ty),
}

impl Expectation {
    pub fn empty() -> Self {
        Expectation::NoValue
    }

    pub fn from_ty(ty: Option<Ty>) -> Self {
        match ty {
            Some(ty) => Expectation::ExpectType(ty),
            None => Expectation::empty(),
        }
    }

    pub fn ty(&self, ctx: &InferenceCtx) -> Option<Ty> {
        match self {
            Expectation::NoValue => None,
            Expectation::ExpectType(ty) => Some(ctx.resolve_vars_if_possible(ty.to_owned())),
        }
    }
}
