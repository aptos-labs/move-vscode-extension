use crate::db::HirDatabase;
use crate::nameres::blocks::get_entries_in_blocks;
use crate::nameres::node_ext::ModuleResolutionExt;
use crate::nameres::scope::{NamedItemsInFileExt, ScopeEntry, ScopeEntryExt};
use crate::nameres::use_speck_entries::use_speck_entries;
use crate::InFile;
use syntax::ast::{GenericItem, HasItems};
use syntax::{ast, AstNode, SyntaxNode};

pub fn get_entries_in_scope(
    db: &dyn HirDatabase,
    scope: InFile<SyntaxNode>,
    prev: Option<SyntaxNode>,
) -> Vec<ScopeEntry> {
    use syntax::SyntaxKind::*;

    let mut entries = vec![];
    if let Some(use_stmts_owner) = ast::AnyHasUseStmts::cast(scope.value.clone()) {
        entries.extend(use_speck_entries(
            db,
            &InFile::new(scope.file_id, use_stmts_owner),
        ));
    }

    if scope.kind() == BLOCK_EXPR {
        entries.extend(get_entries_in_blocks(scope, prev));
        return entries;
    }

    entries.extend(get_entries_from_owner(db, scope));
    entries
}

pub fn get_entries_from_owner(_db: &dyn HirDatabase, scope: InFile<SyntaxNode>) -> Vec<ScopeEntry> {
    use syntax::SyntaxKind::*;

    let file_id = scope.file_id;
    let mut entries = vec![];

    if let Some(has_type_params) = ast::AnyGenericItem::cast(scope.value.clone()) {
        entries.extend(has_type_params.type_params().to_in_file_entries(file_id));
    }

    match scope.kind() {
        MODULE => {
            let module = scope.cast::<ast::Module>().unwrap();
            entries.extend(module.member_entries());
            entries.extend(module.value.enum_variants().to_in_file_entries(file_id));
        }
        MODULE_SPEC => {
            let module_spec = scope.cast::<ast::ModuleSpec>().unwrap();
            entries.extend(module_spec.value.spec_functions().to_in_file_entries(file_id));
            // entries.extend(module_spec.value.spec_inline_functions().to_entries());
            // entries.extend(module_spec.value.schemas().to_entries());
        }
        SCRIPT => {
            let script = scope.cast::<ast::Script>().unwrap();
            entries.extend(script.value.consts().to_in_file_entries(file_id));
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
        FOR_EXPR => {
            let for_expr = scope.cast::<ast::ForExpr>().unwrap();
            let idx_binding = for_expr.value.for_condition().and_then(|it| it.ident_pat());
            if let Some(idx_binding) = idx_binding {
                entries.extend(InFile::new(file_id, idx_binding).to_entry())
            }
        }
        _ => {}
    }

    entries
}
