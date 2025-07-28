// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::item_scope::NamedItemScope;
use crate::loc::SyntaxLocFileExt;
use crate::nameres::path_kind::{PathKind, QualifiedKind, path_kind};
use crate::nameres::scope::ScopeEntry;
use crate::node_ext::has_item_list::HasUseStmtsInFileExt;
use crate::{hir_db, nameres};
use base_db::SourceDatabase;
use syntax::ast;
use syntax::files::{InFile, InFileExt};
use vfs::FileId;

pub fn use_speck_entries(
    db: &dyn SourceDatabase,
    use_stmts_owner: &InFile<impl ast::HasUseStmts>,
) -> Vec<ScopeEntry> {
    let use_items = use_stmts_owner.use_stmt_items(db);

    let mut entries = Vec::with_capacity(use_items.len());
    for use_item in use_items {
        if let Some(entry) = resolve_use_item(db, use_item, use_stmts_owner.file_id) {
            entries.push(entry);
        }
    }

    entries
}

fn resolve_use_item(db: &dyn SourceDatabase, use_item: UseItem, file_id: FileId) -> Option<ScopeEntry> {
    let path = use_item.use_speck.path()?.in_file(file_id);
    let Some(scope_entry) = nameres::resolve_no_inf(db, path.clone()) else {
        tracing::debug!(path = &path.syntax_text(), "cannot resolve use speck");
        return None;
    };
    let node_loc = scope_entry.node_loc;
    Some(ScopeEntry {
        name: use_item.alias_or_name,
        node_loc,
        ns: scope_entry.ns,
        scope_adjustment: Some(use_item.scope),
    })
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

pub fn use_stmt_items(db: &dyn SourceDatabase, use_stmt: InFile<ast::UseStmt>) -> Option<Vec<UseItem>> {
    let root_use_speck = use_stmt.value.use_speck()?;
    let use_stmt_scope = hir_db::item_scope(db, use_stmt.loc());

    let mut use_items = vec![];
    let use_group = root_use_speck.use_group();
    if let Some(use_group) = use_group {
        for child_use_speck in use_group.use_specks() {
            let use_item =
                collect_child_use_speck(root_use_speck.clone(), child_use_speck, use_stmt_scope);
            if let Some(use_item) = use_item {
                use_items.push(use_item);
            }
        }
        return Some(use_items);
    }

    let root_path = root_use_speck.path()?;
    let root_name = root_path.reference_name()?;

    let root_use_speck_alias = root_use_speck.use_alias();
    let root_alias_name = root_use_speck_alias
        .clone()
        .and_then(|alias| alias.name())
        .map(|it| it.as_string());

    let root_path_kind = path_kind(root_path.qualifier(), root_path, false)?;

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
                    return Some(use_items);
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
                }
            }
            _ => {}
        }
    }

    Some(use_items)
}

fn collect_child_use_speck(
    root_use_speck: ast::UseSpeck,
    child_use_speck: ast::UseSpeck,
    use_stmt_scope: NamedItemScope,
) -> Option<UseItem> {
    let qualifier_path = root_use_speck.path()?;
    let module_name = qualifier_path.reference_name()?;

    let child_name = child_use_speck.path()?.reference_name()?;
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

    let qualifier_kind = path_kind(qualifier_path.qualifier(), qualifier_path, false)?;
    // tracing::debug!(qualifier_kind = ?qualifier_kind);

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
