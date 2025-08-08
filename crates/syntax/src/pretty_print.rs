// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::{TextRange, TextSize};
use line_index::LineIndex;
use std::cmp::max;
use std::iter;

pub fn underline_range_in_text(source: &str, text_range: TextRange) -> String {
    let mark = SourceMark::at_range(text_range, "");
    let res = apply_source_marks(source, vec![mark]);
    res
}

pub struct SourceMark {
    pub text_range: TextRange,
    pub message: String,
    pub custom_symbol: Option<String>,
}

impl SourceMark {
    pub fn at_offset(offset: TextSize, message: impl ToString) -> SourceMark {
        SourceMark {
            text_range: TextRange::empty(offset),
            message: message.to_string(),
            custom_symbol: None,
        }
    }

    pub fn at_range(range: TextRange, message: impl ToString) -> SourceMark {
        SourceMark {
            text_range: range,
            message: message.to_string(),
            custom_symbol: None,
        }
    }

    pub fn with_custom_symbol(mut self, symbol: char) -> Self {
        self.custom_symbol = Some(symbol.to_string());
        self
    }
}

pub fn apply_source_marks(source: &str, mut marks: Vec<SourceMark>) -> String {
    let line_index = LineIndex::new(source);

    marks.sort_by_key(|it| it.text_range.start());

    let lines_with_marks = marks
        .into_iter()
        .map(|it| line_with_mark(&line_index, it))
        .collect::<Vec<_>>();

    let mut source_lines = source.lines().map(|it| it.to_string()).collect::<Vec<_>>();
    let mut added = 0;
    for (line, line_text) in lines_with_marks {
        let line = (line + 1 + added) as usize;
        let line_text = line_text.clone();
        if line <= source_lines.len() {
            source_lines.insert(line, line_text);
        } else {
            source_lines.push(line_text);
        }
        added += 1;
    }
    let mut res = source_lines.join("\n");
    res = res.trim_start_matches("\n").trim_end().to_string();
    // add newline at the end
    res.push_str("\n");
    res
}

fn line_with_mark(line_index: &LineIndex, mark: SourceMark) -> (u32, String) {
    let text_range = mark.text_range;
    let lc_start = line_index.line_col(text_range.start());
    let lc_end = line_index.line_col(text_range.end());

    let start_col = lc_start.col;
    let end_col = lc_end.col;
    let message = mark.message;

    let symbol = mark.custom_symbol.unwrap_or("^".into());
    let (prefix, mark_range) = if start_col < 2 {
        let prefix = repeated(" ", start_col);
        let mark_range = "<".to_string();
        (prefix, mark_range)
    } else {
        let prefix = repeated(" ", start_col - 2);
        let mark_range = repeated(&symbol, max(1, end_col.saturating_sub(start_col)));
        // let mark_range = repeated(&symbol, max(1, end_col - start_col));
        (prefix, mark_range)
    };

    let line = format!("{prefix}//{mark_range} {message}").trim_end().to_string();
    (lc_end.line, line)
}

fn repeated(s: &str, n: u32) -> String {
    iter::repeat_n(s, n as usize).collect::<Vec<_>>().join("")
}
