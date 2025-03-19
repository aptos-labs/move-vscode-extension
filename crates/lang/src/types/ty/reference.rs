use crate::types::ty::Ty;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TyReference {
    referenced: Box<Ty>,
    pub is_mut: bool,
}

impl TyReference {
    pub fn new(inner: Ty, is_mut: bool) -> Self {
        TyReference {
            referenced: Box::new(inner),
            is_mut,
        }
    }

    pub fn referenced(&self) -> &Ty {
        self.referenced.as_ref()
    }
}
