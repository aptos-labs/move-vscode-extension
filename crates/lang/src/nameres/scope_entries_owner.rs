use crate::db::HirDatabase;
use crate::files::InFileVecExt;
use crate::nameres::blocks::get_entries_in_blocks;
use crate::nameres::node_ext::ModuleResolutionExt;
use crate::nameres::scope::{NamedItemsExt, ScopeEntry};
use crate::nameres::use_speck_entries::use_speck_entries;
use crate::InFile;
use parser::SyntaxKind::{BLOCK_EXPR, MODULE};
use syntax::ast::{HasItemList, HasTypeParams};
use syntax::{ast, AstNode, SyntaxNode};

pub fn get_entries_in_scope(
    db: &dyn HirDatabase,
    scope: InFile<SyntaxNode>,
    prev: Option<SyntaxNode>,
) -> Vec<ScopeEntry> {
    use syntax::SyntaxKind::*;

    if let BLOCK_EXPR | SPEC_BLOCK_EXPR = scope.kind() {
        return get_entries_in_blocks(scope, prev);
    }

    if ast::AnyHasScopeEntries::can_cast(scope.kind()) {
        return get_entries_from_owner(db, scope);
    }

    vec![]
}

pub fn get_entries_from_owner(db: &dyn HirDatabase, scope: InFile<SyntaxNode>) -> Vec<ScopeEntry> {
    use syntax::SyntaxKind::*;

    let mut entries = vec![];
    let file_id = scope.file_id;
    match scope.kind() {
        MODULE => {
            let module = scope.cast::<ast::Module>().unwrap();
            entries.extend(module.member_entries());
            entries.extend(
                module
                    .value
                    .enum_variants()
                    .wrapped_in_file(module.file_id)
                    .to_entries(),
            );
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
            entries.extend(script.value.consts().wrapped_in_file(script.file_id).to_entries());
            // entries.extend(script.consts().to_entries());
            // use
            entries.extend(use_speck_entries(db, &script));
        }
        FUN => {
            let fun = scope.cast::<ast::Fun>().unwrap();
            // let fun = ast::Fun::cast(scope.value).unwrap();
            entries.extend(fun.value.type_params().wrapped_in_file(fun.file_id).to_entries());
            entries.extend(fun.value.params_as_bindings().wrapped_in_file(fun.file_id).to_entries());
        }
        // SCHEMA => {
        //     let schema = ast::Schema::cast(scope.value).unwrap();
        //     entries.extend(schema.schema_fields_as_bindings().to_entries())
        // }
        _ => {
            unreachable!("{:?} scope entries owner is not handled", scope.kind());
        }
    }

    entries
}
