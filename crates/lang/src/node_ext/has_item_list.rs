use crate::InFile;
use crate::nameres::use_speck_entries::{UseItem, use_stmt_items};
use syntax::ast;
//
// pub trait HasItemListExt {
//     fn use_stmt_items(&self, file_id: FileId) -> Vec<UseItem>;
// }
//
// impl<T: ast::HasItemList> HasItemListExt for T {
//     fn use_stmt_items(&self, file_id: FileId) -> Vec<UseItem> {
//         self.use_stmts()
//             .into_iter()
//             .flat_map(|it| use_stmt_items(it, file_id))
//             .collect()
//     }
// }

pub trait HasUseStmtsInFileExt {
    fn use_stmt_items(&self) -> Vec<UseItem>;

    // fn items(&self) -> Vec<InFile<ast::Item>>;
    // fn consts(&self) -> Vec<InFile<ast::Const>>;
    // fn functions(&self) -> Vec<InFile<ast::Fun>>;
}

impl<T: ast::HasUseStmts> HasUseStmtsInFileExt for InFile<T> {
    fn use_stmt_items(&self) -> Vec<UseItem> {
        self.value
            .use_stmts()
            .into_iter()
            .flat_map(|it| use_stmt_items(it, self.file_id))
            .collect()
    }

    // fn items(&self) -> Vec<InFile<ast::Item>> {
    //     self.value.items().wrapped_in_file(self.file_id)
    // }
    //
    // fn consts(&self) -> Vec<InFile<ast::Const>> {
    //     self.value.consts().wrapped_in_file(self.file_id)
    // }
    //
    // fn functions(&self) -> Vec<InFile<ast::Fun>> {
    //     self.value.functions().wrapped_in_file(self.file_id)
    // }
}
