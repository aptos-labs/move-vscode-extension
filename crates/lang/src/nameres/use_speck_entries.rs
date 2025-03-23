use crate::db::HirDatabase;
use crate::files::InFileInto;
use crate::nameres::path_kind::{path_kind, PathKind, QualifiedKind};
use crate::nameres::scope::ScopeEntry;
use crate::node_ext::has_item_list::HasUseStmtsInFileExt;
use crate::node_ext::PathLangExt;
use crate::InFile;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxNodeExt;
use syntax::ast::{NamedElement, NamedItemScope};
use syntax::{ast, AstNode};
use vfs::FileId;

pub fn use_speck_entries(
    db: &dyn HirDatabase,
    items_owner: &InFile<impl ast::HasUseStmts>,
) -> Vec<ScopeEntry> {
    let use_items = items_owner.use_stmt_items();

    let mut entries = vec![];
    for use_item in use_items {
        let path = InFile::new(items_owner.file_id, use_item.use_speck.path());
        let Some(scope_entry) = path.clone().resolve_no_inf(db) else {
            tracing::debug!(path = &path.syntax_text(), "use_speck unresolved");
            continue;
        };
        let node_loc = scope_entry.node_loc;
        entries.push(ScopeEntry {
            name: use_item.alias_or_name,
            node_loc,
            ns: scope_entry.ns,
            scope_adjustment: Some(use_item.scope),
        });
    }

    entries
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum UseItemType {
    Module,
    SelfModule,
    Item,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct UseItem {
    use_speck: ast::UseSpeck,
    use_alias: Option<ast::UseAlias>,
    alias_or_name: String,
    type_: UseItemType,
    scope: NamedItemScope,
}

pub fn use_stmt_items(use_stmt: ast::UseStmt, file_id: FileId) -> Vec<UseItem> {
    #[rustfmt::skip]
    let Some(root_use_speck) = use_stmt.use_speck() else { return vec![]; };

    let use_stmt_scope = use_stmt.syntax().item_scope();
    let mut use_items = vec![];

    let use_group = root_use_speck.use_group();
    if let Some(use_group) = use_group {
        for child_use_speck in use_group.use_specks() {
            let use_item = collect_child_use_speck(
                root_use_speck.clone(),
                child_use_speck,
                file_id,
                use_stmt_scope,
            );
            if let Some(use_item) = use_item {
                use_items.push(use_item);
            }
        }
        return use_items;
    }

    #[rustfmt::skip]
    let Some(root_name) = root_use_speck.path().reference_name() else { return use_items; };

    let root_use_speck_alias = root_use_speck.use_alias();
    let root_alias_name = root_use_speck_alias
        .clone()
        .and_then(|alias| alias.name())
        .map(|it| it.as_string());

    let root_path = root_use_speck.path();
    let root_path_kind = path_kind(InFile::new(file_id, root_path), false);
    // tracing::debug!(root_path_kind = ?root_path_kind);

    if let PathKind::Qualified { qualifier, kind, .. } = root_path_kind {
        match kind {
            // use 0x1::m;
            // use aptos_std::m;
            QualifiedKind::Module { .. } => use_items.push(UseItem {
                use_speck: root_use_speck,
                use_alias: root_use_speck_alias,
                alias_or_name: root_alias_name.unwrap_or(root_name),
                type_: UseItemType::Module,
                scope: use_stmt_scope,
            }),
            // use 0x1::m::call;
            // use aptos_std::m::call as mycall;
            // use aptos_std::m::Self;
            QualifiedKind::FQModuleItem { .. } => {
                let Some(module_name) = qualifier.reference_name() else {
                    return use_items;
                };
                if root_name.as_str() == "Self" {
                    use_items.push(UseItem {
                        use_speck: root_use_speck,
                        use_alias: root_use_speck_alias,
                        alias_or_name: root_alias_name.unwrap_or(module_name),
                        type_: UseItemType::SelfModule,
                        scope: use_stmt_scope,
                    });
                } else {
                    use_items.push(UseItem {
                        use_speck: root_use_speck,
                        use_alias: root_use_speck_alias,
                        alias_or_name: root_alias_name.unwrap_or(root_name),
                        type_: UseItemType::Item,
                        scope: use_stmt_scope,
                    });
                    // let base_path = path.base_path();
                    // let address = match path_kind(base_path, false) {
                    //     Some(PathKind::NamedAddress(named_address)) => {
                    //         Some(Address::Named(named_address))
                    //     }
                    //     Some(PathKind::ValueAddress(value_address)) => {
                    //         Some(Address::Value(value_address))
                    //     }
                    //     _ => None,
                    // };
                    // if let Some(address) = address {
                    // }
                }
            }
            _ => {}
        }
    }

    use_items
}

fn collect_child_use_speck(
    root_use_speck: ast::UseSpeck,
    child_use_speck: ast::UseSpeck,
    file_id: FileId,
    use_stmt_scope: NamedItemScope,
) -> Option<UseItem> {
    let qualifier_path = root_use_speck.path();
    let module_name = qualifier_path.reference_name()?;

    let child_name = child_use_speck.path().reference_name()?;
    let child_alias = child_use_speck.use_alias();
    let child_alias_name = child_alias
        .clone()
        .and_then(|alias| alias.name())
        .map(|name| name.as_string());

    if child_name.as_str() == "Self" {
        return Some(UseItem {
            use_speck: child_use_speck,
            use_alias: child_alias,
            alias_or_name: child_alias_name.unwrap_or(module_name),
            type_: UseItemType::SelfModule,
            scope: use_stmt_scope,
        });
    }

    let qualifier_kind = path_kind(InFile::new(file_id, qualifier_path), false);
    tracing::debug!(qualifier_kind = ?qualifier_kind);

    if let PathKind::Qualified { .. } = qualifier_kind {
        // let address = match kind {
        //     QualifiedKind::Module { address, .. } => address,
        //     QualifiedKind::ModuleOrItem { address, .. } => address,
        //     _ => {
        //         return None;
        //     }
        // };
        let child_name_or_alias = child_alias_name.unwrap_or(child_name);
        return Some(UseItem {
            use_speck: child_use_speck,
            use_alias: child_alias,
            alias_or_name: child_name_or_alias,
            type_: UseItemType::Item,
            scope: use_stmt_scope,
        });
    }

    None
}
