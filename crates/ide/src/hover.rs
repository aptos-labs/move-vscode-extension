use crate::RangeInfo;
use ide_db::helpers::pick_best_token;
use ide_db::RootDatabase;
use lang::{FilePosition, Semantics};
use syntax::{AstNode, SyntaxNode, T};

/// Contains the results when hovering over an item
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct HoverResult {
    pub doc_string: String,
    // pub actions: Vec<HoverAction>,
}

// Feature: Hover
//
// Shows additional information, like the type of an expression or the documentation for a definition when "focusing" code.
// Focusing is usually hovering with a mouse, but can also be triggered with a shortcut.
//
// ![Hover](https://user-images.githubusercontent.com/48062697/113020658-b5f98b80-917a-11eb-9f88-3dbc27320c95.gif)
pub(crate) fn hover(db: &RootDatabase, file_position: FilePosition) -> Option<RangeInfo<HoverResult>> {
    let sema = &Semantics::new(db);
    let file = sema.parse(file_position.file_id).syntax().clone();

    let res = hover_offset(sema, file_position, file);

    res
}

fn hover_offset(
    sema: &Semantics<'_, RootDatabase>,
    FilePosition { file_id, offset }: FilePosition,
    file: SyntaxNode,
) -> Option<RangeInfo<HoverResult>> {
    use syntax::SyntaxKind::*;

    let original_token = pick_best_token(file.token_at_offset(offset), |kind| match kind {
        IDENT | INT_NUMBER | T!['_'] => 4,
        // index and prefix ops and closure pipe
        T!['['] | T![']'] | T![*] | T![-] | T![!] | T![|] => 3,
        kind if kind.is_keyword() => 2,
        T!['('] | T![')'] => 2,
        kind if kind.is_trivia() => 0,
        _ => 1,
    })?;

    None
}
