use crate::nameres;
use base_db::SourceDatabase;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};

pub trait ItemSpecExt {
    fn item(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::Item>>;
}

impl ItemSpecExt for InFile<ast::ItemSpec> {
    fn item(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::Item>> {
        let item_spec_ref = self.and_then_ref(|it| it.item_spec_ref())?;
        let entry = nameres::resolve(db, item_spec_ref)?;
        entry.cast_into::<ast::Item>(db)
    }
}

pub fn get_item_spec_function(
    db: &dyn SourceDatabase,
    result_path_expr: InFile<&ast::PathExpr>,
) -> Option<InFile<ast::Fun>> {
    let (file_id, path_expr) = result_path_expr.unpack();
    let item_spec = path_expr.syntax().ancestor_or_self::<ast::ItemSpec>()?;

    let item = item_spec.in_file(file_id).item(db)?;
    item.cast_into::<ast::Fun>()
}
