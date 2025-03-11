use crate::db::HirDatabase;
use crate::nameres::blocks::get_entries_in_blocks;
use crate::nameres::node_ext::ModuleResolutionExt;
use crate::nameres::scope::{NamedItemsExt, ScopeEntry};
use crate::nameres::use_speck_entries::use_speck_entries;
use parser::SyntaxKind::{BLOCK_EXPR, MODULE};
use syntax::ast::{HasItemList, HasTypeParams};
use syntax::{ast, AstNode, SyntaxNode};

pub fn get_entries_in_scope(
    db: &dyn HirDatabase,
    scope: SyntaxNode,
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

pub fn get_entries_from_owner(db: &dyn HirDatabase, scope: SyntaxNode) -> Vec<ScopeEntry> {
    use syntax::SyntaxKind::*;

    let mut entries = vec![];
    match scope.kind() {
        MODULE => {
            let module = ast::Module::cast(scope).unwrap();
            entries.extend(module.member_entries());
            entries.extend(module.enum_variants().to_entries());
            // use
            entries.extend(use_speck_entries(db, &module));
        }
        MODULE_SPEC => {
            let module_spec = ast::ModuleSpec::cast(scope).unwrap();
            entries.extend(module_spec.spec_functions().to_entries());
            entries.extend(module_spec.spec_inline_functions().to_entries());
            entries.extend(module_spec.schemas().to_entries());
            // use
            entries.extend(use_speck_entries(db, &module_spec));
        }
        SCRIPT => {
            let script = ast::Script::cast(scope).unwrap();
            entries.extend(script.consts().to_entries());
            // use
            entries.extend(use_speck_entries(db, &script));
        }
        FUN => {
            let fun = ast::Fun::cast(scope).unwrap();
            entries.extend(fun.type_params().to_entries());
            entries.extend(fun.params_as_bindings().to_entries());
        }
        SCHEMA => {
            let schema = ast::Schema::cast(scope).unwrap();
            entries.extend(schema.schema_fields_as_bindings().to_entries())
        }
        _ => {
            unreachable!("{:?} scope entries owner is not handled", scope.kind());
        }
    }

    entries
}
