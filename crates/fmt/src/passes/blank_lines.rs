use crate::engine::fmt_model::{FmtBlock, FmtBlockModel, ws_indent};
use syntax::{SyntaxKind, SyntaxKind::*, T};

pub(crate) fn normalize_blank_lines(model: &mut FmtBlockModel) {
    model.for_each_block_mut(normalize_blank_lines_in_block);
}

fn normalize_blank_lines_in_block(parent: &mut FmtBlock) {
    if parent.children().is_empty() {
        return;
    }

    let parent_kind = parent.kind();
    let mut after_l_curly = false;
    let mut prev_block_kind: Option<SyntaxKind> = None;

    for child_block in parent.child_blocks_mut() {
        let child_kind = child_block.kind();

        let ws = child_block.ws_before_mut();
        clamp_blank_lines(ws, parent_kind, prev_block_kind, child_kind, after_l_curly);

        if child_kind == L_CURLY {
            after_l_curly = true;
        }
        prev_block_kind = Some(child_kind);
    }
}

/// Clamp blank lines in a WS entry, preserving the existing indent.
fn clamp_blank_lines(
    ws: &mut String,
    parent_kind: SyntaxKind,
    prev_kind: Option<SyntaxKind>,
    next_kind: SyntaxKind,
    after_l_curly: bool,
) {
    let num_newlines = ws.chars().filter(|&c| c == '\n').count();
    if num_newlines == 0 {
        return;
    }

    let existing_blank_lines = num_newlines - 1;
    let (min_blank, max_blank) = blank_line_bounds(parent_kind, prev_kind, next_kind, after_l_curly);
    let desired_blank_lines = existing_blank_lines.clamp(min_blank, max_blank);

    if desired_blank_lines == existing_blank_lines {
        return;
    }

    // Preserve the indent (spaces after the last \n).
    let new_ws = format!(
        "{}\n{}",
        "\n".repeat(desired_blank_lines),
        " ".repeat(ws_indent(ws))
    );
    *ws = new_ws;
}

/// Returns (min, max) allowed blank lines between two siblings.
pub(crate) fn blank_line_bounds(
    parent_kind: SyntaxKind,
    prev_kind: Option<SyntaxKind>,
    // it's always there as we only check for preceding ws
    next_kind: SyntaxKind,
    after_l_curly: bool,
) -> (usize, usize) {
    match (prev_kind, next_kind) {
        // Block edge: no blank lines.
        (None, _) => (0, 0),

        (Some(T!['{']), _) => (0, 0),
        (_, T!['}']) => (0, 0),

        // Between items inside flat blocks: enforce exactly 1
        // (unless exempted by `needs_blank_line_between`).
        (Some(prev), next)
            if after_l_curly && is_flat_block(parent_kind) && needs_blank_line_between(prev, next) =>
        {
            (1, 1)
        }
        // Everything else: cap at 1.
        (Some(_), _) => (0, 1),
    }
}

/// Adapted from IntelliJ's `needsBlankLineBetweenItems`.
fn needs_blank_line_between(prev: SyntaxKind, next: SyntaxKind) -> bool {
    // Comments attach to the adjacent item — don't force blank lines around them.
    if prev == COMMENT || next == COMMENT {
        return false;
    }
    // Consecutive one-line items of the same kind can stay together.
    if is_one_line_item(prev) && prev == next {
        return false;
    }
    true
}

fn is_one_line_item(kind: SyntaxKind) -> bool {
    matches!(kind, USE_STMT | CONST | FRIEND)
}

fn is_flat_block(kind: SyntaxKind) -> bool {
    matches!(kind, MODULE | SCRIPT | MODULE_SPEC)
}
