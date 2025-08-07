// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::nameres::use_speck_entries::{UseItem, use_items_for_stmt};
use base_db::SourceDatabase;
use syntax::ast;
use syntax::files::InFile;

pub trait HasUseStmtsInFileExt {
    fn use_items(&self, db: &dyn SourceDatabase) -> Vec<UseItem>;
}

impl<T: ast::HasUseStmts> HasUseStmtsInFileExt for InFile<T> {
    fn use_items(&self, db: &dyn SourceDatabase) -> Vec<UseItem> {
        let stmts = self.clone().flat_map(|it| it.use_stmts().collect());
        stmts
            .into_iter()
            .flat_map(|stmt| use_items_for_stmt(db, stmt).unwrap_or_default())
            .collect()
    }
}
