use syntax::ast;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TyTypeParameter {
    pub origin: ast::TypeParam,
}

impl TyTypeParameter {
    pub fn new(origin: ast::TypeParam) -> Self {
        TyTypeParameter { origin }
    }
}
