use crate::nameres::use_speck_entries::{use_stmt_items, UseItem};
use crate::{AsName, Name};
use syntax::ast;

pub trait PathExt {
    fn name_ref_name(&self) -> Option<Name>;
}

impl PathExt for ast::Path {
    fn name_ref_name(&self) -> Option<Name> {
        self.name_ref().map(|name_ref| name_ref.as_name())
    }
}

pub trait HasItemListExt {
    fn use_stmt_items(&self) -> Vec<UseItem>;
}

impl<T: ast::HasItemList> HasItemListExt for T {
    fn use_stmt_items(&self) -> Vec<UseItem> {
        self.use_stmts()
            .into_iter()
            .flat_map(|it| use_stmt_items(it))
            .collect()
    }
}
