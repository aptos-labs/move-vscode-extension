use crate::types::ty::Ty;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TyTuple {
    pub types: Vec<Ty>,
}

impl TyTuple {
    pub fn new(types: Vec<Ty>) -> Self {
        TyTuple { types }
    }
}
