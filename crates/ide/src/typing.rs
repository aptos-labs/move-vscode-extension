use base_db::inputs::InternFileId;
use base_db::source_db;
use ide_db::RootDatabase;
use ide_db::source_change::SourceChange;
use ide_db::text_edit::TextEdit;
use syntax::algo::ancestors_at_offset;
use syntax::files::FilePosition;
use syntax::{AstNode, Parse, SourceFile, SyntaxKind, TextSize, ast};

// Don't forget to add new trigger characters to `server_capabilities` in `caps.rs`.
pub(crate) const TRIGGER_CHARS: &[char] = &['<'];

pub(crate) fn on_char_typed(
    db: &RootDatabase,
    position: FilePosition,
    char_typed: char,
) -> Option<SourceChange> {
    if !TRIGGER_CHARS.contains(&char_typed) {
        return None;
    }

    let file = source_db::parse(db, position.file_id.intern(db));
    let char_matches_position = file.tree().syntax().text().char_at(position.offset) == Some(char_typed);
    if !stdx::always!(char_matches_position) {
        return None;
    }

    let edit = on_char_typed_(&file, position.offset, char_typed)?;
    Some(SourceChange::from_text_edit(position.file_id, edit))
}

fn on_char_typed_(file: &Parse, offset: TextSize, char_typed: char) -> Option<TextEdit> {
    match char_typed {
        '<' => on_opening_delimiter_typed(file, offset, char_typed),
        _ => None,
    }
}

fn on_opening_delimiter_typed(
    file: &Parse,
    offset: TextSize,
    opening_bracket: char,
) -> Option<TextEdit> {
    // todo: other braces, see rust-analyzer
    let expected_ast_bracket = match opening_bracket {
        '<' => SyntaxKind::L_ANGLE,
        _ => return None,
    };

    let brace_token = file.tree().syntax().token_at_offset(offset).right_biased()?;
    if brace_token.kind() != expected_ast_bracket {
        return None;
    }

    // Remove the opening bracket to get a better parse tree, and reparse.
    let range = brace_token.text_range();
    if !stdx::always!(range.len() == TextSize::of(opening_bracket)) {
        return None;
    }
    let reparsed = file.reparse(range, "").tree();

    match opening_bracket {
        '<' => on_left_angle_typed(&file.tree(), &reparsed, offset),
        _ => None,
    }
}

/// Add closing `>` for generic arguments/parameters.
fn on_left_angle_typed(file: &SourceFile, reparsed: &SourceFile, offset: TextSize) -> Option<TextEdit> {
    let file_text = reparsed.syntax().text();

    // Find the next non-whitespace char in the line, check if its a `>`
    let mut next_offset = offset;
    while file_text.char_at(next_offset) == Some(' ') {
        next_offset += TextSize::of(' ')
    }
    if file_text.char_at(next_offset) == Some('>') {
        return None;
    }

    if ancestors_at_offset(file.syntax(), offset)
        .take_while(|n| !ast::Item::can_cast(n.kind()))
        .any(|n| ast::TypeParamList::can_cast(n.kind()) || ast::TypeArgList::can_cast(n.kind()))
    {
        // Insert the closing bracket right after
        Some(TextEdit::insert(offset + TextSize::of('<'), "$0>".to_string()))
    } else {
        None
    }
}
