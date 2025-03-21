use crate::db::HirDatabase;
use base_db::input::CrateId;
use syntax::ast;

/// hir::Crate describes a single crate. It's the main interface with which
/// a crate's dependencies interact. Mostly, it should be just a proxy for the
/// root module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Crate {
    pub(crate) id: CrateId,
}

#[derive(Debug)]
pub struct CrateDependency {
    pub name: String,
    pub krate: Crate,
}

impl Crate {
    pub fn modules(self, _db: &dyn HirDatabase) -> Vec<ast::Module> {
        // todo: implement and cache inside db later
        vec![]
    }
}
