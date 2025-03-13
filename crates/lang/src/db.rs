use crate::loc::SyntaxLoc;
use crate::nameres::paths;
use crate::nameres::scope::ScopeEntry;
use crate::InFile;
use base_db::{SourceDatabase, SourceRootDatabase, Upcast};
use parser::SyntaxKind;
use std::ops::Deref;
use syntax::{ast, AstNode, SyntaxNode, TextRange, TextSize};
use triomphe::Arc;
use vfs::FileId;

#[ra_salsa::query_group(HirDatabaseStorage)]
pub trait HirDatabase: SourceRootDatabase + Upcast<dyn SourceRootDatabase> {
    fn resolve_ast_path(&self, path: SyntaxLoc) -> Vec<ScopeEntry>;
}

fn resolve_ast_path(db: &dyn HirDatabase, path: SyntaxLoc) -> Vec<ScopeEntry> {
    #[rustfmt::skip]
    let Some(path) = path.cast::<ast::Path>(db.upcast()) else { return vec![]; };
    paths::resolve(db, path)
}
