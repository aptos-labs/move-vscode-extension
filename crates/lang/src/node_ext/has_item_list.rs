use crate::HirDatabase;
use crate::nameres::use_speck_entries::{UseItem, use_stmt_items};
use syntax::ast;
use syntax::files::InFile;

pub trait HasUseStmtsInFileExt {
    fn use_stmt_items(&self, db: &dyn HirDatabase) -> Vec<UseItem>;
}

impl<T: ast::HasUseStmts> HasUseStmtsInFileExt for InFile<T> {
    fn use_stmt_items(&self, db: &dyn HirDatabase) -> Vec<UseItem> {
        let stmts = self.clone().flat_map(|it| it.use_stmts().collect());
        stmts
            .into_iter()
            .flat_map(|stmt| use_stmt_items(db, stmt).unwrap_or_default())
            .collect()
    }
}
