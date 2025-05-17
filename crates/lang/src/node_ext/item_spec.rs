use crate::nameres::ResolveReference;
use base_db::SourceDatabase;
use syntax::ast;
use syntax::files::InFile;

pub trait ItemSpecExt {
    fn item(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::Item>>;
}

impl ItemSpecExt for InFile<ast::ItemSpec> {
    fn item(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::Item>> {
        let item_spec_ref = self.and_then_ref(|it| it.item_spec_ref())?;
        let resolved = item_spec_ref.resolve(db)?;
        resolved.cast_into::<ast::Item>(db)
    }
}
