// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::references::find_def_at_offset;
use ide_db::search::SearchScope;
use ide_db::{RootDatabase, search};
use lang::Semantics;
use std::collections::HashSet;
use syntax::files::FilePosition;
use syntax::{AstNode, TextRange};

pub(crate) fn highlight_related<'db>(
    sema: &'db Semantics<'db, RootDatabase>,
    FilePosition { offset, file_id }: FilePosition,
) -> Option<Vec<TextRange>> {
    let _p = tracing::info_span!("highlight_related").entered();

    let tree = sema.parse(file_id).syntax().clone();
    let def = find_def_at_offset(sema, &tree, offset)?;
    let def_name = def.value.name()?;

    let mut res: HashSet<TextRange> = HashSet::default();
    if def.file_id == file_id {
        res.insert(def_name.syntax().text_range());
    }

    let usages = search::item_usages(sema, def)
        .in_scope(SearchScope::from_single_file(file_id))
        .fetch_all()
        .references
        .remove(&file_id)
        .unwrap_or_default();
    res.extend(usages.iter().map(|it| it.range));

    if res.is_empty() {
        None
    } else {
        Some(res.into_iter().collect())
    }
}
