use crate::config::CstFormatConfig;
use syntax::SyntaxKind;
use syntax::SyntaxKind::*;

use crate::engine::fmt_model::{Direction, FmtBlock, FmtBlockModel, has_line_break};
use crate::engine::spacing_rules::{Spacing, SpacingRules};
use crate::rules::spacing::build_spacing_rules;

pub(crate) fn normalize_spacing(model: &mut FmtBlockModel, config: &CstFormatConfig) {
    let rules = SpacingRules::new(build_spacing_rules());
    walk_spacing(model.root_mut(), &rules, config, 0, resolve_required_spacing);
}

/// Insert all optional line breaks within `scope` in a single pass.
pub(crate) fn insert_optional_line_breaks(model: &mut FmtBlockModel, config: &CstFormatConfig) {
    let root = model.root_mut();
    let rules = SpacingRules::new(build_spacing_rules());
    walk_spacing(root, &rules, config, 0, resolve_optional_line_break);
}

/// Resolve required spacing: Spaces and LineBreak apply directly,
/// SpacesOrLineBreak/DependentLineBreak treated as spaces (optional
/// breaks handled in pass 2).
fn resolve_required_spacing(
    spacing: &Spacing,
    _line_len: &dyn Fn() -> usize,
    _parent: &FmtBlock,
    _config: &CstFormatConfig,
) -> Option<Spacing> {
    match spacing {
        Spacing::Spaces(_) | Spacing::LineBreak | Spacing::SpacesOrPreserveLineBreak(_) => {
            Some(spacing.clone())
        }
        Spacing::SpacesOrLineBreak(spaces) | Spacing::DependentLineBreak(spaces) => {
            Some(Spacing::Spaces(*spaces))
        }
    }
}

/// Resolve optional line breaks: check if the line is too long or if the
/// parent already has a break (for DependentLineBreak).
fn resolve_optional_line_break(
    spacing: &Spacing,
    current_line_len: &dyn Fn() -> usize,
    parent: &FmtBlock,
    config: &CstFormatConfig,
) -> Option<Spacing> {
    match spacing {
        Spacing::SpacesOrLineBreak(spaces) => {
            if current_line_len() + spaces > config.max_line_width() {
                return Some(Spacing::LineBreak);
            }
            None
        }
        Spacing::DependentLineBreak(spaces) => {
            // if anything in the parent subtree already broke, break here too.
            if parent.children_ws().any(has_line_break) {
                return Some(Spacing::LineBreak);
            }

            // otherwise, check for the optional line break
            if current_line_len() + spaces > config.max_line_width() {
                return Some(Spacing::LineBreak);
            }

            None
        }
        Spacing::LineBreak | Spacing::Spaces(_) | Spacing::SpacesOrPreserveLineBreak(_) => None,
    }
}

/// Generic tree walk. For each WS between siblings with a matching spacing
/// rule, calls `resolve` to get a decision, then applies it to the WS.
///
/// Resolve borrows the parent immutably; apply borrows children mutably.
/// These borrows alternate per-iteration — no overlap.
fn walk_spacing(
    parent: &mut FmtBlock,
    rules: &SpacingRules,
    config: &CstFormatConfig,
    parent_indent: usize,
    resolve_spacing: fn(&Spacing, &dyn Fn() -> usize, &FmtBlock, &CstFormatConfig) -> Option<Spacing>,
) {
    let parent_kind = parent.kind();

    if parent.children().is_empty() {
        return;
    }

    let child_indexes = 0..parent.children().len();

    // --- Phase 1: resolve and apply all WS at this level ---
    let mut prev_block_kind: Option<SyntaxKind> = None;

    for child_idx in child_indexes.clone() {
        let block = parent.child_block(child_idx).unwrap();
        let block_kind = block.kind();

        if let Some(matched_rule) = rules.find_matching_rule(parent_kind, prev_block_kind, block_kind) {
            let spacing_rule = {
                let line_len = || line_len_at(parent, child_idx);
                let spacing_rule = resolve_spacing(&matched_rule.spacing, &line_len, parent, config);
                spacing_rule
            };

            if let Some(spacing_rule) = spacing_rule {
                let block = parent.child_block_mut(child_idx).unwrap();
                apply_spacing(block, &spacing_rule, parent_indent, config);
            }
        }

        prev_block_kind = Some(block_kind);
    }

    // --- Phase 2: recurse after all WS at this level has been resolved. ---
    // Example:
    // /*parent_indent*/1
    //     /*actual indent we need to use to get the correct line len*/==> 2 == 3
    let mut current_line_indent = parent_indent;
    for child in parent.child_blocks_mut() {
        if child.ws_has_line_break() {
            current_line_indent = parent_indent + config.indent(child.indent_type());
        }

        walk_spacing(child, rules, config, current_line_indent, resolve_spacing);
    }
}

/// Apply resolved spacing before `block`.
/// Returns true if a line break was inserted or acknowledged.
fn apply_spacing(
    block: &mut FmtBlock,
    spacing: &Spacing,
    parent_indent: usize,
    config: &CstFormatConfig,
) -> bool {
    let child_indent = parent_indent + config.indent(block.indent_type());
    let ws = block.ws_before_mut();

    match spacing {
        Spacing::Spaces(count) => {
            let expected = " ".repeat(*count);
            if *ws != expected {
                *ws = expected;
            }
            false
        }
        Spacing::LineBreak => {
            if has_line_break(ws) {
                // Preserve existing newline count, fix the indent.
                // Blank line clamping is handled by the blank lines pass.
                let newlines: usize = ws.chars().filter(|&c| c == '\n').count();
                let new_text = format!("{}{}", "\n".repeat(newlines), " ".repeat(child_indent));
                if *ws != new_text {
                    *ws = new_text;
                }
            } else {
                *ws = format!("\n{}", " ".repeat(child_indent));
            }
            true
        }
        Spacing::SpacesOrPreserveLineBreak(count) => {
            if has_line_break(ws) {
                apply_spacing(block, &Spacing::LineBreak, parent_indent, config)
            } else {
                apply_spacing(block, &Spacing::Spaces(*count), parent_indent, config)
            }
        }
        Spacing::SpacesOrLineBreak(_) | Spacing::DependentLineBreak(_) => {
            unreachable!("should be resolved before apply")
        }
    }
}

/// Compute the current line length at `children[child_idx].ws_before`.
/// Walks left and right from the WS using `leaves_from`, summing text
/// lengths until hitting a `\n` in each direction.
fn line_len_at(parent: &FmtBlock, child_idx: usize) -> usize {
    parent.line_len_at(child_idx, Direction::RightToLeft)
        + parent.line_len_at(child_idx, Direction::LeftToRight)
}
