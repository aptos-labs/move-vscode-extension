use crate::loc::SyntaxLoc;
use crate::nameres::paths;
use crate::nameres::scope::ScopeEntry;
use base_db::{SourceRootDatabase, Upcast};
use syntax::ast;

#[ra_salsa::query_group(HirDatabaseStorage)]
pub trait HirDatabase: SourceRootDatabase + Upcast<dyn SourceRootDatabase> {
    fn resolve_ast_path(&self, path: SyntaxLoc) -> Vec<ScopeEntry>;
}

fn resolve_ast_path(db: &dyn HirDatabase, path: SyntaxLoc) -> Vec<ScopeEntry> {
    #[rustfmt::skip]
    let Some(path) = path.cast::<ast::Path>(db.upcast()) else { return vec![]; };
    paths::resolve(db, path)
}
