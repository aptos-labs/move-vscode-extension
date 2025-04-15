use crate::db::HirDatabase;
use crate::types::ty::Ty;
use syntax::ast;

// todo: move to Semantics?
pub trait TyFromInferenceExt {
    fn ty(&self, db: &dyn HirDatabase) -> Option<Ty>;
}

impl TyFromInferenceExt for ast::IdentPat {
    fn ty(&self, db: &dyn HirDatabase) -> Option<Ty> {
        let owner = self.owner()?;
        match owner {
            // ast::IdentPatOwner::Param(param) => {}
            _ => None,
        }
    }
}
