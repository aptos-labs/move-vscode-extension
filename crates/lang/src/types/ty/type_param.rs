use crate::loc::{SyntaxLoc, SyntaxLocFileExt};
use base_db::SourceDatabase;
use syntax::ast;
use syntax::files::InFile;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TyTypeParameter {
    pub origin_loc: SyntaxLoc,
}

impl TyTypeParameter {
    pub fn new(origin: InFile<ast::TypeParam>) -> Self {
        TyTypeParameter { origin_loc: origin.loc() }
    }

    pub fn from_loc(origin_loc: SyntaxLoc) -> Self {
        TyTypeParameter { origin_loc }
    }

    pub fn origin_type_param(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::TypeParam>> {
        self.origin_loc.to_ast::<ast::TypeParam>(db)
    }
}
