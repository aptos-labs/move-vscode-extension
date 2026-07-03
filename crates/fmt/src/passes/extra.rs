use crate::config::CstFormatConfig;
use crate::engine::fmt_model::{Direction, FmtBlock, FmtBlockModel, has_line_break};
use syntax::SyntaxKind::{BIN_EXPR, COMMENT, IMPLY_INCLUDE_EXPR, LET_STMT, SCHEMA_LIT, STRUCT_LIT};
use syntax::T;

// block can only have a leading comment if it's in a doc comment place
// // my doc comment
// fun main() {}
pub(crate) fn normalize_leading_comments(model: &mut FmtBlockModel) {
    model.for_each_block_mut(|block| {
        // if block has a leading comment (either doc comment or plain one)
        if let Some(leading_comment) = block.children_mut().first_mut().filter(|it| it.kind() == COMMENT)
        {
            // normalize it's leading ws to ""
            leading_comment.set_ws("");
        }
    });
}

/// If `=` in a LET_STMT is followed by a BIN_EXPR that already contains
/// line breaks (from chain breaking), collapse the `=\n  ` to `= ` so the
/// first operand stays on the `=` line.
pub(crate) fn collapse_let_eq_before_broken_bin_expr(
    model: &mut FmtBlockModel,
    config: &CstFormatConfig,
) {
    model.for_each_descendant_pattern_mut(
        Some(LET_STMT),
        T![=],
        BIN_EXPR,
        |let_stmt_block, _, rhs_idx| {
            dedent_multiline_bin_expr_at_rhs(let_stmt_block, rhs_idx, &config);
        },
    );
}

pub(crate) fn collapse_let_eq_before_broken_struct_lit(
    model: &mut FmtBlockModel,
    config: &CstFormatConfig,
) {
    model.for_each_descendant_pattern_mut(
        Some(LET_STMT),
        T![=],
        STRUCT_LIT,
        |let_stmt_block, _, rhs_idx| {
            dedent_multiline_struct_lit_at_rhs(let_stmt_block, rhs_idx, &config);
        },
    );
}

pub(crate) fn collapse_imply_include_rhs_struct_lit(
    model: &mut FmtBlockModel,
    config: &CstFormatConfig,
) {
    model.for_each_descendant_pattern_mut(
        Some(IMPLY_INCLUDE_EXPR),
        T![==>],
        SCHEMA_LIT,
        |imply_include_expr, implies_idx, rhs_idx| {
            indent_schema_lit_after_implies(imply_include_expr, implies_idx, rhs_idx, &config);
        },
    );
}

fn dedent_multiline_bin_expr_at_rhs(
    let_stmt: &mut FmtBlock,
    rhs_idx: usize,
    config: &CstFormatConfig,
) -> Option<()> {
    let rhs_block = let_stmt.child_block_mut(rhs_idx)?;
    if !rhs_block.children_ws().any(has_line_break) {
        return None;
    }

    rhs_block.set_ws(" ");
    rhs_block.dedent_by(config.continuation_indent());

    Some(())
}

fn dedent_multiline_struct_lit_at_rhs(
    let_stmt: &mut FmtBlock,
    rhs_idx: usize,
    config: &CstFormatConfig,
) -> Option<()> {
    let rhs_block = let_stmt.child_block(rhs_idx)?;
    if !rhs_block
        .leafs(Direction::LeftToRight)
        .any(|leaf| leaf.has_line_break())
    {
        return None;
    }
    // needs to be
    // let =
    //     STRUCT_LIT
    if !rhs_block.ws_has_line_break() {
        return None;
    }

    let line_len_at_ws = let_stmt.line_len_at(rhs_idx, Direction::RightToLeft)
        + 1
        + let_stmt.line_len_at(rhs_idx, Direction::LeftToRight);
    if line_len_at_ws > config.max_line_width() {
        return None;
    }

    let rhs_block = let_stmt.child_block_mut(rhs_idx)?;
    rhs_block.set_ws(" ");
    rhs_block.dedent_by(config.continuation_indent());

    Some(())
}

// IMPLY_INCLUDE_EXPR:
//   <include condition expr>
//   WS
//   IMPLIES (`==>`)
//   WS
//   SCHEMA_LIT
//     PATH
//     SCHEMA_LIT_FIELD_LIST
fn indent_schema_lit_after_implies(
    imply_include_expr: &mut FmtBlock,
    implies_idx: usize,
    rhs_idx: usize,
    config: &CstFormatConfig,
) -> Option<()> {
    // there's no line break before `==>`, don't do anything
    if !imply_include_expr.child_block(implies_idx)?.ws_has_line_break() {
        return None;
    }

    // include dual_attestation
    //     ==> DualAttestation::AssertPaymentOkAbortsIf<Token> {
    //             value: amount
    //         };
    let rhs_block = imply_include_expr.child_block_mut(rhs_idx)?;
    rhs_block.indent_by(config.continuation_indent());

    Some(())
}
