use crate::types::inference::InferenceCtx;
use crate::types::ty::Ty;

#[derive(Debug, Clone)]
pub enum Expected {
    NoValue,
    ExpectType(Ty),
}

impl Expected {
    pub fn from_ty(ty: Option<Ty>) -> Self {
        match ty {
            Some(ty) => Expected::ExpectType(ty),
            None => Expected::NoValue,
        }
    }

    pub fn ty(&self, ctx: &InferenceCtx) -> Option<Ty> {
        match self {
            Expected::NoValue => None,
            Expected::ExpectType(ty) => Some(ctx.resolve_ty_vars_if_possible(ty.to_owned())),
        }
    }
}
