use crate::nameres::use_speck_entries::{UseItem, use_stmt_items};
use syntax::ast;
use syntax::files::InFile;

pub trait HasUseStmtsInFileExt {
    fn use_stmt_items(&self) -> Vec<UseItem>;
}

impl<T: ast::HasUseStmts> HasUseStmtsInFileExt for InFile<T> {
    fn use_stmt_items(&self) -> Vec<UseItem> {
        self.value
            .use_stmts()
            .into_iter()
            .flat_map(|it| use_stmt_items(it).unwrap_or_default())
            .collect()
    }
}
