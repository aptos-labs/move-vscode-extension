pub mod fold;
pub mod resolve;

use line_index::LineCol;
use syntax::TextSize;

pub struct MarkedPos {
    pub mark_offset: TextSize,
    pub item_offset: TextSize,
    pub mark_line_col: LineCol,
    pub item_line_col: LineCol,
    pub line: String,
    pub data: String,
}

pub fn get_all_marked_positions(source: &str, mark: &str) -> Vec<MarkedPos> {
    let mut positions = vec![];
    let file_index = line_index::LineIndex::new(source);
    let pattern = Regex::new(&regex::escape(mark)).unwrap();
    for match_ in pattern.find_iter(source) {
        let match_offset = match_.start();
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
    let file_index = line_index::LineIndex::new(source);
    let LineCol { line, col } = file_index.line_col(TextSize::new(offset));
    let ref_line = line - 1; // it's a //^ comment underneath the element
    let ref_col = col + 2; // we need a position of ^
    (ref_line, ref_col)
}

pub fn get_marked_position_offset_with_data(source: &str, mark: &str) -> (TextSize, String) {
    let marked = get_first_marked_position(source, mark);
    (marked.item_offset, marked.data)
}

/// Asserts that two strings are equal, otherwise displays a rich diff between them.
///
/// The diff shows changes from the "original" left string to the "actual" right string.
///
/// All arguments starting from and including the 3rd one are passed to
/// `eprintln!()` macro in case of text inequality.
#[macro_export]
macro_rules! assert_eq_text {
    ($left:expr, $right:expr) => {
        assert_eq_text!($left, $right,)
    };
    ($left:expr, $right:expr, $($tt:tt)*) => {{
        let left = $left;
        let right = $right;
        if left != right {
            if left.trim() == right.trim() {
                std::eprintln!("Left:\n{:?}\n\nRight:\n{:?}\n\nWhitespace difference\n", left, right);
            } else {
                let diff = $crate::test_utils::__diff(left, right);
                std::eprintln!("Left:\n{}\n\nRight:\n{}\n\nDiff:\n{}\n", left, right, $crate::test_utils::format_diff(diff));
            }
            std::eprintln!($($tt)*);
            panic!("text differs");
        }
    }};
}

pub use dissimilar::diff as __diff;
use regex::Regex;

pub fn format_diff(chunks: Vec<dissimilar::Chunk<'_>>) -> String {
    let mut buf = String::new();
    for chunk in chunks {
        let formatted = match chunk {
            dissimilar::Chunk::Equal(text) => text.into(),
            dissimilar::Chunk::Delete(text) => format!("\x1b[41m{text}\x1b[0m"),
            dissimilar::Chunk::Insert(text) => format!("\x1b[42m{text}\x1b[0m"),
        };
        buf.push_str(&formatted);
    }
    buf
}
