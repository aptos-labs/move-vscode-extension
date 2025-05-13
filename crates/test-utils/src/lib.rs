pub mod fixtures;

use line_index::{LineCol, LineIndex};
use regex::Regex;
use std::iter;
use syntax::{TextRange, TextSize};

pub fn get_and_replace_caret(source: &str, caret_mark: &str) -> (&'static str, TextSize) {
    let caret_offset = source
        .find(caret_mark)
        .expect(&format!("{} not found", caret_mark));
    let source_no_caret = source.replace(caret_mark, "");
    (source_no_caret.leak(), TextSize::new(caret_offset as u32))
}

pub struct Marking {
    pub text_range: TextRange,
    pub message: String,
}

pub fn remove_markings(source: &str) -> String {
    let marked_positions = get_all_marked_positions(source, "//^");

    let mut lines_to_remove = vec![];
    for marked in marked_positions {
        lines_to_remove.push(marked.mark_line_col.line as usize);
    }

    let trimmed_source = source
        .lines()
        .enumerate()
        .filter(|(i, line)| !lines_to_remove.contains(i))
        .map(|it| it.1)
        .collect::<Vec<_>>()
        .join("\n");

    trimmed_source
}

pub fn apply_markings(source: &str, markings: Vec<Marking>) -> String {
    let line_index = LineIndex::new(source);

    let mut lines = vec![];
    for marking in markings {
        let text_range = marking.text_range;
        let lc_start = line_index.line_col(text_range.start());
        let lc_end = line_index.line_col(text_range.end());
        let line = marked_line(marking.message, lc_start.col, lc_end.col);
        lines.push((lc_start.line, line));
    }

    let mut source_lines = source.lines().map(|it| it.to_string()).collect::<Vec<_>>();
    let mut added = 0;
    for (line, line_text) in lines {
        let line = line + 1 + added;
        source_lines.insert(line as usize, line_text.clone());
        added += 1;
    }
    source_lines.join("\n")
}

fn marked_line(message: String, start_col: u32, end_col: u32) -> String {
    let prefix = iter::repeat_n(" ", (start_col - 2) as usize)
        .collect::<Vec<_>>()
        .join("");
    let range = iter::repeat_n("^", (end_col - start_col) as usize)
        .collect::<Vec<_>>()
        .join("");
    let line = format!("{prefix}//{range} {message}");
    line
}

pub struct MarkedPos {
    pub mark_offset: TextSize,
    pub item_offset: TextSize,
    pub mark_line_col: LineCol,
    pub item_line_col: LineCol,
    pub line: String,
    pub data: String,
}

pub fn get_all_marked_positions(source: &str, mark: &str) -> Vec<MarkedPos> {
    let file_index = LineIndex::new(source);
    let pattern = Regex::new(&regex::escape(mark)).unwrap();

    let mut positions = vec![];
    for m in pattern.find_iter(source) {
        let match_offset = m.start();
        let LineCol { line, col } = file_index.line_col(TextSize::new(match_offset as u32));
        let ref_line = line - 1; // it's a //^ comment underneath the element
        let ref_col = col + 2; // we need a position of ^
        let line_text = source
            .chars()
            .skip(match_offset)
            .collect::<String>()
            .lines()
            .next()
            .map(|it| it.to_string())
            .unwrap();
        let item_line_col = LineCol { line: ref_line, col: ref_col };
        let offset = file_index.offset(item_line_col).unwrap();
        let data = line_text.trim_start_matches(mark).trim().to_string();
        positions.push(MarkedPos {
            mark_offset: TextSize::from(match_offset as u32),
            item_offset: offset,
            mark_line_col: LineCol { line, col },
            item_line_col: LineCol { line: ref_line, col: ref_col },
            line: line_text,
            data,
        });
    }
    positions
}

pub fn get_first_marked_position(source: &str, mark: &str) -> MarkedPos {
    let marked_pos = get_all_marked_positions(source, mark)
        .pop()
        .expect(&format!("no positions marked with {mark:?} found in file source"));
    marked_pos
}

pub fn get_marked_position(source: &str, mark: &str) -> (u32, u32) {
    let offset = source
        .find(mark)
        .expect(&format!("No `{}` found in the source file", mark)) as u32;
    let file_index = LineIndex::new(source);
    let LineCol { line, col } = file_index.line_col(TextSize::new(offset));
    let ref_line = line - 1; // it's a //^ comment underneath the element
    let ref_col = col + 2; // we need a position of ^
    (ref_line, ref_col)
}

pub fn get_marked_position_offset_with_data(source: &str, mark: &str) -> (TextSize, String) {
    let marked = get_first_marked_position(source, mark);
    (marked.item_offset, marked.data)
}
