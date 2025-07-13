// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{NavigationTarget, RangeInfo};
use base_db::inputs::InternFileId;
use ide_db::RootDatabase;
use ide_db::helpers::pick_best_token;
use lang::loc::SyntaxLocFileExt;
use lang::{Semantics, item_specs};
use syntax::SyntaxKind::IDENT;
use syntax::files::{FilePosition, InFileExt};
use syntax::{AstNode, algo, ast};

pub(crate) fn goto_specification(
    db: &RootDatabase,
    FilePosition { file_id, offset }: FilePosition,
) -> Option<RangeInfo<Vec<NavigationTarget>>> {
    let sema = Semantics::new(db, file_id);
    let source_file = sema.parse(file_id);
    let syntax = source_file.syntax().clone();

    let original_token = pick_best_token(syntax.token_at_offset(offset), |kind| match kind {
        IDENT => 1,
        _ => 0,
    })?;
    let range = original_token.text_range();

    let fun = algo::find_node_at_offset::<ast::Fun>(&syntax, offset)?.in_file(file_id);

    let item_spec_map = item_specs::get_item_specs_for_items_in_file(db, file_id.intern(db));
    let fun_item_specs = item_spec_map.get(&fun.loc())?;

    let navs = fun_item_specs
        .into_iter()
        .filter_map(|loc| NavigationTarget::from_syntax_loc(db, "spec".into(), loc.clone()))
        .collect::<Vec<_>>();

    Some(RangeInfo { range, info: navs })
}
