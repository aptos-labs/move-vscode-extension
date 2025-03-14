use crate::db::HirDatabase;
use crate::nameres::blocks::get_entries_in_blocks;
use crate::nameres::node_ext::ModuleResolutionExt;
use crate::nameres::scope::{NamedItemsInFileExt, ScopeEntry};
use crate::nameres::use_speck_entries::use_speck_entries;
use crate::InFile;
use syntax::ast::{HasItemList, HasTypeParams};
use syntax::{ast, AstNode, SyntaxNode};

pub fn get_entries_in_scope(
    db: &dyn HirDatabase,
    scope: InFile<SyntaxNode>,
    prev: Option<SyntaxNode>,
) -> Vec<ScopeEntry> {
    use syntax::SyntaxKind::*;

    if let BLOCK_EXPR = scope.kind() {
        return get_entries_in_blocks(scope, prev);
    }

    get_entries_from_owner(db, scope)
}

pub fn get_entries_from_owner(db: &dyn HirDatabase, scope: InFile<SyntaxNode>) -> Vec<ScopeEntry> {
    use syntax::SyntaxKind::*;

    let file_id = scope.file_id;
    let mut entries = vec![];

    if let Some(has_type_params) = ast::AnyHasTypeParams::cast(scope.value.clone()) {
        entries.extend(has_type_params.type_params().to_in_file_entries(file_id));
    }

    match scope.kind() {
        MODULE => {
            let module = scope.cast::<ast::Module>().unwrap();
            entries.extend(module.member_entries());
            entries.extend(module.value.enum_variants().to_in_file_entries(file_id));
            // use
            entries.extend(use_speck_entries(db, &module));
        }
        MODULE_SPEC => {
            let module_spec = scope.cast::<ast::ModuleSpec>().unwrap();
            // entries.extend(module_spec.value.spec_functions().to_entries());
            // entries.extend(module_spec.value.spec_inline_functions().to_entries());
            // entries.extend(module_spec.value.schemas().to_entries());
            // use
            entries.extend(use_speck_entries(db, &module_spec));
        }
        SCRIPT => {
            let script = scope.cast::<ast::Script>().unwrap();
            entries.extend(script.value.consts().to_in_file_entries(file_id));
            // use
            entries.extend(use_speck_entries(db, &script));
        }
        FUN => {
            let fun = scope.cast::<ast::Fun>().unwrap();
            entries.extend(fun.value.params_as_bindings().to_in_file_entries(file_id));
        }
        SCHEMA => {
            let schema = scope.cast::<ast::Schema>().unwrap();
            entries.extend(
                schema
                    .value
                    .schema_fields_as_bindings()
                    .to_in_file_entries(file_id),
            )
        }
        _ => {}
    }

    entries
}
