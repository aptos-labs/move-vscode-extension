use crate::RangeInfo;
use crate::navigation_target::NavigationTarget;
use ide_db::RootDatabase;
use ide_db::helpers::pick_best_token;
use lang::Semantics;
use lang::nameres::scope::VecExt;
use syntax::files::FilePosition;
use syntax::{AstNode, SyntaxKind::*, T, algo, ast};

pub(crate) fn goto_definition(
    db: &RootDatabase,
    file_position: FilePosition,
) -> Option<RangeInfo<NavigationTarget>> {
    let RangeInfo {
        range: reference_range,
        info: targets,
    } = goto_definition_multi(db, file_position)?;

    let nav_target = targets.single_or_none()?;

    Some(RangeInfo::new(reference_range, nav_target))
}

pub(crate) fn goto_definition_multi(
    db: &RootDatabase,
    FilePosition { file_id, offset }: FilePosition,
) -> Option<RangeInfo<Vec<NavigationTarget>>> {
    let sema = Semantics::new(db, file_id);

    let file = sema.parse(file_id);

    let reference = algo::find_node_at_offset::<ast::ReferenceElement>(file.syntax(), offset)?;
    let original_token = pick_best_token(file.syntax().token_at_offset(offset), |kind| match kind {
        IDENT | QUOTE_IDENT | INT_NUMBER | COMMENT => 4,
        // index and prefix ops
        T!['['] | T![']'] | T![*] | T![-] | T![!] => 3,
        kind if kind.is_keyword() => 2,
        T!['('] | T![')'] => 2,
        kind if kind.is_trivia() => 0,
        _ => 1,
    })?;

    let scope_entries = sema.resolve(reference);

    let nav_targets = scope_entries
        .into_iter()
        .filter_map(|it| NavigationTarget::from_scope_entry(&sema, it))
        .collect::<Vec<_>>();

    Some(RangeInfo::new(original_token.text_range(), nav_targets))
}
