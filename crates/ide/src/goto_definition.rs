use crate::RangeInfo;
use crate::navigation_target::NavigationTarget;
use ide_db::RootDatabase;
use ide_db::helpers::pick_best_token;
use lang::Semantics;
use lang::nameres::scope::VecExt;
use syntax::files::FilePosition;
use syntax::{AstNode, SyntaxKind::*, T, algo, ast};

// Feature: Go to Definition
//
// Navigates to the definition of an identifier.
//
// For outline modules, this will navigate to the source file of the module.
//
// |===
// | Editor  | Shortcut
//
// | VS Code | kbd:[F12]
// |===
//
// image::https://user-images.githubusercontent.com/48062697/113065563-025fbe00-91b1-11eb-83e4-a5a703610b23.gif[]
pub(crate) fn goto_definition(
    db: &RootDatabase,
    FilePosition { file_id, offset }: FilePosition,
) -> Option<RangeInfo<NavigationTarget>> {
    let sema = Semantics::new(db, file_id);

    let file = sema.parse(file_id);

    let reference = algo::find_node_at_offset::<ast::AnyReferenceElement>(file.syntax(), offset)?;
    let scope_entry = sema.resolve(reference).single_or_none()?;

    let original_token = pick_best_token(file.syntax().token_at_offset(offset), |kind| match kind {
        IDENT
        | QUOTE_IDENT
        | INT_NUMBER
        | COMMENT => 4,
        // index and prefix ops
        T!['['] | T![']'] /*| T![?] */| T![*] | T![-] | T![!] => 3,
        kind if kind.is_keyword() => 2,
        T!['('] | T![')'] => 2,
        kind if kind.is_trivia() => 0,
        _ => 1,
    })?;

    let nav_info = NavigationTarget::from_scope_entry(db, scope_entry)?;
    Some(RangeInfo::new(original_token.text_range(), nav_info))
}
