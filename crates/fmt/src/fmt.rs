// Copyright © Aptos Foundation
// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::config::CstFormatConfig;
use crate::engine::fmt_model::FmtBlockModel;
use crate::passes::{blank_lines, extra, spacing, trailing_commas};
use syntax::ast;

pub fn format_content(content: &str, config: CstFormatConfig) -> Result<String, String> {
    let parse = ast::SourceFile::parse(content);
    let file = parse.tree();

    // Trailing commas — before spacing so `after(COMMA)` doesn't insert a space
    // next to a comma that is about to be removed.
    let file = trailing_commas::remove_trailing_commas_in_file(file, &config);

    // Build block model for spacing + optional line breaks passes.
    let mut fmt_model = FmtBlockModel::new(&file);

    // Pass 1: normalize whitespace (spaces and required line breaks only, no optional breaks).
    spacing::normalize_spacing(&mut fmt_model, &config);

    // Pass 2: insert optional line breaks one item at a time until stable.
    spacing::insert_optional_line_breaks(&mut fmt_model, &config);

    // If `=` in a LET_STMT is followed by a BIN_EXPR that already contains
    // line breaks, collapse the `=\n` to `= ` so the first operand stays
    // on the `=` line.
    extra::collapse_let_eq_before_broken_bin_expr(&mut fmt_model, &config);
    extra::collapse_let_eq_before_broken_struct_lit(&mut fmt_model, &config);
    extra::collapse_imply_include_rhs_struct_lit(&mut fmt_model, &config);

    extra::normalize_leading_comments(&mut fmt_model);

    // Clamp blank lines.
    blank_lines::normalize_blank_lines(&mut fmt_model);

    let result = fmt_model.into_text().trim().to_string() + "\n";
    Ok(result)
}
