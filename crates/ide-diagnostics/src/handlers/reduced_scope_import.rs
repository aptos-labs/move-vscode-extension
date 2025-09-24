use crate::handlers::unused_import::use_items;
use base_db::SourceDatabase;
use lang::hir_db;
use lang::item_scope::ItemScope;
use lang::nameres::use_speck_entries::{UseItem, use_items_for_stmt};
use std::collections::HashSet;
use stdx::itertools::Itertools;
use syntax::ast;
use syntax::files::InFile;

pub(crate) fn find_use_items_with_redundant_main_scope(
    db: &dyn SourceDatabase,
    use_stmts_owner: &InFile<ast::AnyUseStmtsOwner>,
) -> Option<Vec<UseItem>> {
    let _p = tracing::debug_span!("find_use_items_to_reduce").entered();

    let use_items_hit_with_usage_scopes = use_items::find_use_items_hit_with_scopes(db, use_stmts_owner);
    // #[verify_only] usages for this diagnostics have same meaning as #[main]
    let use_items_hit_with_usage_scopes = use_items_hit_with_usage_scopes
        .into_iter()
        .map(|(use_item, scope)| match scope {
            ItemScope::Verify => (use_item, ItemScope::Main),
            _ => (use_item, scope),
        })
        .collect::<HashSet<_>>();

    let mut use_items_with_unused_declared_scopes = vec![];
    for use_stmt in hir_db::combined_use_stmts(db, use_stmts_owner) {
        for use_item in use_items_for_stmt(db, use_stmt)? {
            let use_items_with_declared_scopes = match &use_item.declared_scope {
                // `use 0x1::m` -> [(use 0x1::m, Main), (use 0x1::m, Test)]
                ItemScope::Main => HashSet::from([
                    (use_item.clone(), ItemScope::Main),
                    (use_item.clone(), ItemScope::Test),
                ]),
                // `#[test_only] use 0x1::m` -> [(use 0x1::m, Test)]
                ItemScope::Test => HashSet::from([(use_item.clone(), ItemScope::Test)]),
                ItemScope::Verify => {
                    // #[verify_only] does not participate in scope reduction
                    continue;
                }
            };
            use_items_with_unused_declared_scopes.extend(
                use_items_with_declared_scopes
                    .difference(&use_items_hit_with_usage_scopes)
                    .cloned(),
            );
        }
    }

    let mut use_items_with_reductable_scope = vec![];
    for (use_item, chunk) in &use_items_with_unused_declared_scopes
        .iter()
        .chunk_by(|(use_item, _)| use_item)
    {
        let scopes = chunk.map(|(_, scope)| *scope).collect::<Vec<_>>();
        // only #[main] -> #[test_only] reduction is supported
        if scopes == vec![ItemScope::Main] {
            use_items_with_reductable_scope.push(use_item.clone());
        }
    }

    Some(use_items_with_reductable_scope)
}
