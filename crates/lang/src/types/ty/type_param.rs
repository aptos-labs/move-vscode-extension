use syntax::ast;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TyTypeParameter {
    pub origin: ast::TypeParam,
}
