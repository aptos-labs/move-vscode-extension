// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::item_scope::ItemScope;
use crate::loc::{SyntaxLoc, SyntaxLocFileExt, SyntaxLocNodeExt};
use crate::nameres::path_kind::{PathKind, QualifiedKind, path_kind};
use crate::nameres::scope::ScopeEntry;
use crate::{hir_db, nameres};
use base_db::SourceDatabase;
use syntax::ast;
use syntax::files::InFile;
use vfs::FileId;

pub fn use_speck_entries(
    db: &dyn SourceDatabase,
    use_stmts_owner: InFile<ast::AnyUseStmtsOwner>,
) -> Vec<ScopeEntry> {
    let use_items = hir_db::use_items(db, use_stmts_owner.clone());
    let mut entries = Vec::with_capacity(use_items.len());
    for use_item in use_items {
        if let Some(entry) = resolve_use_item(db, use_item) {
            entries.push(entry);
        }
    }

    entries
}

fn resolve_use_item(db: &dyn SourceDatabase, use_item: UseItem) -> Option<ScopeEntry> {
    let path = use_item
        .use_speck_loc
        .to_ast::<ast::UseSpeck>(db)?
        .and_then(|it| it.path())?;
    let Some(scope_entry) = nameres::resolve_no_inf(db, path.clone()) else {
        tracing::debug!(path = &path.syntax_text(), "cannot resolve use speck");
        return None;
    };
    let node_loc = scope_entry.node_loc;
    Some(ScopeEntry {
        name: use_item.alias_or_name,
        node_loc,
        ns: scope_entry.ns,
        scope_adjustment: Some(use_item.declared_scope),
    })
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum UseItemType {
    Module,
    SelfModule,
    Item,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct UseItem {
    pub use_speck_loc: SyntaxLoc,
    pub alias_or_name: String,
    pub type_: UseItemType,
    pub declared_scope: ItemScope,
}

impl UseItem {
    pub fn use_speck(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::UseSpeck>> {
        self.use_speck_loc.to_ast(db)
    }
}

pub fn use_items_for_stmt(
    db: &dyn SourceDatabase,
    use_stmt: InFile<ast::UseStmt>,
) -> Option<Vec<UseItem>> {
    let use_stmt_scope = hir_db::item_scope(db, use_stmt.loc());

    let (file_id, use_stmt) = use_stmt.unpack();
    let root_use_speck = use_stmt.use_speck()?;

    let mut use_items = vec![];
    let use_group = root_use_speck.use_group();
    if let Some(use_group) = use_group {
        for child_use_speck in use_group.use_specks() {
            let use_item = collect_child_use_speck(
                db,
                root_use_speck.clone(),
                child_use_speck,
                file_id,
                use_stmt_scope,
            );
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

    let root_path_kind = path_kind(db, root_path.qualifier(), &root_path, false)?;

    if let PathKind::Qualified { qualifier, kind, .. } = root_path_kind {
        match kind {
            // use 0x1::m;
            // use aptos_std::m;
            QualifiedKind::Module { .. } => use_items.push(UseItem {
                use_speck_loc: root_use_speck.loc(file_id),
                alias_or_name: root_alias_name.unwrap_or(root_name),
                type_: UseItemType::Module,
                declared_scope: use_stmt_scope,
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
                        use_speck_loc: root_use_speck.loc(file_id),
                        alias_or_name: root_alias_name.unwrap_or(module_name),
                        type_: UseItemType::SelfModule,
                        declared_scope: use_stmt_scope,
                    });
                } else {
                    use_items.push(UseItem {
                        use_speck_loc: root_use_speck.loc(file_id),
                        alias_or_name: root_alias_name.unwrap_or(root_name),
                        type_: UseItemType::Item,
                        declared_scope: use_stmt_scope,
                    });
                }
            }
            _ => {}
        }
    }

    Some(use_items)
}

fn collect_child_use_speck(
    db: &dyn SourceDatabase,
    root_use_speck: ast::UseSpeck,
    child_use_speck: ast::UseSpeck,
    file_id: FileId,
    use_stmt_scope: ItemScope,
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
            use_speck_loc: child_use_speck.loc(file_id),
            alias_or_name: child_alias_name.unwrap_or(module_name),
            type_: UseItemType::SelfModule,
            declared_scope: use_stmt_scope,
        });
    }

    let qualifier_kind = path_kind(db, qualifier_path.qualifier(), &qualifier_path, false)?;
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
            use_speck_loc: child_use_speck.loc(file_id),
            alias_or_name: child_name_or_alias,
            type_: UseItemType::Item,
            declared_scope: use_stmt_scope,
        });
    }

    None
}
