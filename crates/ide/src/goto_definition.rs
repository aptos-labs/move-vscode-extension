use crate::navigation_target::NavigationTarget;
use crate::RangeInfo;
use ide_db::helpers::pick_best_token;
use ide_db::RootDatabase;
use lang::files::FilePosition;
use lang::Semantics;
use syntax::{algo, ast, AstNode, SyntaxKind::*, T};

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
    let sema = Semantics::new(db);

    let file = sema.parse(file_id);
    let path = algo::find_node_at_offset::<ast::Path>(file.syntax(), offset)?;

    let scope_entry = sema.resolve_path(path)?;

    let original_token = pick_best_token(file.syntax().token_at_offset(offset), |kind| match kind {
        IDENT
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
