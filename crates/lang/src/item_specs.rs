use crate::loc::{SyntaxLoc, SyntaxLocFileExt};
use crate::nameres::node_ext::ModuleResolutionExt;
use crate::node_ext::item_spec::ItemSpecExt;
use base_db::inputs::{FileIdInput, InternFileId};
use base_db::{SourceDatabase, source_db};
use std::collections::HashMap;
use syntax::ast::HasItems;
use syntax::files::InFileExt;

pub fn get_item_specs_for_fun(db: &dyn SourceDatabase, fun_loc: SyntaxLoc) -> Vec<SyntaxLoc> {
    get_item_specs_for_items_in_file(db, fun_loc.file_id().intern(db))
        .get(&fun_loc)
        .cloned()
        .unwrap_or_default()
}

#[salsa_macros::tracked(returns(ref))]
pub fn get_item_specs_for_items_in_file(
    db: &dyn SourceDatabase,
    file_id: FileIdInput,
) -> HashMap<SyntaxLoc, Vec<SyntaxLoc>> {
    let source_file = source_db::parse(db, file_id).tree();
    let file_id = file_id.data(db);

    let modules = source_file
        .all_modules()
        .map(|it| it.in_file(file_id))
        .collect::<Vec<_>>();

    let mut items_with_item_specs = HashMap::new();
    for module in modules {
        let mut module_item_specs = module.as_ref().flat_map(|it| it.all_item_specs());

        let related_module_specs = module.related_module_specs(db);
        for module_spec in related_module_specs {
            let item_specs = module_spec.flat_map(|it| it.all_item_specs());
            module_item_specs.extend(item_specs);
        }

        for item_spec in module_item_specs {
            if let Some(item) = item_spec.item(db) {
                let entries = items_with_item_specs.entry(item.loc()).or_insert(vec![]);
                entries.push(item_spec.loc());
            }
        }
    }
    items_with_item_specs
}
