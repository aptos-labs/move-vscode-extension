use base_db::SourceDatabase;
use lang::hir_db;
use lang::item_scope::ItemScope;
use lang::loc::SyntaxLocFileExt;
use lang::nameres::use_speck_entries::{UseItem, UseItemType};
use std::collections::HashSet;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::files::InFile;
use syntax::{AstNode, ast};

pub(crate) fn find_use_items_hit_with_scopes(
    db: &dyn SourceDatabase,
    use_stmts_owner: &InFile<ast::AnyUseStmtsOwner>,
) -> HashSet<(UseItem, ItemScope)> {
    let reachable_paths = hir_db::reachable_paths(db, use_stmts_owner);
    let mut use_items_hit = HashSet::new();
    for path in reachable_paths {
        if let Some(use_item_with_scope) = find_use_item_hit_for_path(db, path.clone()) {
            use_items_hit.insert(use_item_with_scope);
        }
    }
    use_items_hit
}

fn find_use_item_hit_for_path(
    db: &dyn SourceDatabase,
    path: InFile<ast::Path>,
) -> Option<(UseItem, ItemScope)> {
    let _p = tracing::debug_span!("find_use_item_hit_for_path").entered();

    let path_scope = hir_db::item_scope(db, path.loc());
    let specific_path_item_scope = if path_scope != ItemScope::Main {
        Some(path_scope)
    } else {
        None
    };

    let use_item_owner_ancestors = path
        .as_ref()
        .flat_map(|it| it.syntax().ancestors_of_type::<ast::AnyUseStmtsOwner>());

    let base_path_type = BasePathType::for_path(db, &path.value)?;
    for use_item_owner_ans in use_item_owner_ancestors {
        let owner_use_items = hir_db::combined_use_items(db, use_item_owner_ans)
            .into_iter()
            .filter(|use_item| {
                use_item.declared_scope == ItemScope::Main
                    || specific_path_item_scope.is_some_and(|it| use_item.declared_scope == it)
            });
        let use_items_hit_in_owner = find_use_items_hit(owner_use_items, &base_path_type);

        // first try to find a candidate in the specific scope, then in main scope
        let use_item_hit = specific_path_item_scope
            .and_then(|path_scope| {
                use_items_hit_in_owner
                    .iter()
                    .find(|item| item.declared_scope == path_scope)
            })
            .or_else(|| {
                use_items_hit_in_owner
                    .iter()
                    .find(|item| item.declared_scope == ItemScope::Main)
            })
            .cloned();

        if use_item_hit.is_some() {
            return use_item_hit.map(|it| (it, path_scope));
        }
    }
    None
}

fn find_use_items_hit(
    use_items: impl Iterator<Item = UseItem>,
    path_type: &BasePathType,
) -> Vec<UseItem> {
    match path_type {
        BasePathType::Item { item_name } => use_items
            .filter(|it| it.type_ == UseItemType::Item && it.alias_or_name.eq(item_name))
            .collect(),
        BasePathType::Module { module_name } => use_items
            .filter(|it| {
                matches!(it.type_, UseItemType::Module | UseItemType::SelfModule)
                    && it.alias_or_name.eq(module_name)
            })
            .collect(),
        _ => vec![],
    }
}

#[derive(Debug)]
enum BasePathType {
    Address,
    Module { module_name: String },
    Item { item_name: String },
}

impl BasePathType {
    fn for_path(db: &dyn SourceDatabase, path: &ast::Path) -> Option<BasePathType> {
        let root_path = path.root_path();
        let qualifier = root_path.qualifier();
        match qualifier {
            // foo
            None => Some(BasePathType::Item {
                item_name: root_path.reference_name()?,
            }),
            // 0x1::foo
            Some(qualifier) if qualifier.path_address().is_some() => Some(BasePathType::Address),
            Some(qualifier) => {
                let parent_qualifier = qualifier.qualifier();
                // addr::m::foo
                if let Some(parent_qualifier) = parent_qualifier
                    && let Some(parent_name) = parent_qualifier.reference_name()
                    && hir_db::named_addresses(db).contains_key(&parent_name)
                {
                    Some(BasePathType::Address)
                } else {
                    // mod::foo
                    Some(BasePathType::Module {
                        module_name: qualifier.reference_name()?,
                    })
                }
            }
        }
    }
}
