use crate::HirDatabase;
use crate::nameres::ResolveReference;
use syntax::ast;
use syntax::files::InFile;

pub trait ItemSpecExt {
    fn item(&self, db: &dyn HirDatabase) -> Option<InFile<ast::Item>>;
}

impl ItemSpecExt for InFile<ast::ItemSpec> {
    fn item(&self, db: &dyn HirDatabase) -> Option<InFile<ast::Item>> {
        let item_spec_ref = self.and_then_ref(|it| it.item_spec_ref())?;
        let resolved = item_spec_ref.resolve_no_inf(db)?;
        resolved.cast_into::<ast::Item>(db)
    }
}
