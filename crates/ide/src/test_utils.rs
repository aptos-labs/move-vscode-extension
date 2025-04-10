pub mod fold;
pub mod resolve;

use line_index::LineCol;
use syntax::TextSize;

pub fn get_and_replace_caret(source: &str, caret_mark: &str) -> (&'static str, TextSize) {
    let caret_offset = source
        .find(caret_mark)
        .expect(&format!("{} not found", caret_mark));
    let source_no_caret = source.replace(caret_mark, "");
    (source_no_caret.leak(), TextSize::new(caret_offset as u32))
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

pub fn get_marked_position_offset(source: &str, mark: &str) -> TextSize {
    let (line, col) = get_marked_position(source, mark);
    let file_index = line_index::LineIndex::new(source);
    let offset = file_index.offset(LineCol { line, col }).unwrap();
    TextSize::new(offset.into())
}

pub fn get_marked_position_line_index(source: &str, mark: &str) -> usize {
    // let position_offset = get_marked_position_offset(source, mark);

    let (line_idx, _) = source
        .lines()
        .enumerate()
        .find(|(i, line)| line.contains(mark))
        .expect(&format!("no {} mark", mark));
    line_idx
    //
    // let offset = source.find(mark).unwrap();
    // let trimmed_source = source.chars().skip(offset).collect::<String>();
    // let line = trimmed_source.lines().next().map(|it| it.to_string());
    //
    // (position_offset, line.unwrap_or("".to_string()))
}

pub fn get_marked_position_offset_with_line(source: &str, mark: &str) -> (TextSize, String) {
    let position_offset = get_marked_position_offset(source, mark);

    let offset = source.find(mark).unwrap();
    let trimmed_source = source.chars().skip(offset).collect::<String>();
    let line = trimmed_source.lines().next().map(|it| it.to_string());

    (position_offset, line.unwrap_or("".to_string()))
}

pub fn get_marked_position_offset_with_data(source: &str, mark: &str) -> (TextSize, String) {
    let (offset, line) = get_marked_position_offset_with_line(source, mark);

    let data = line.trim_start_matches(mark).trim();
    (offset, data.to_string())
    // let position_offset = get_marked_position_offset(source, mark);
    //
    // let offset = source.find(mark).unwrap();
    // let trimmed_source = source.chars().skip(offset).collect::<String>();
    // let data = trimmed_source
    //     .trim_start_matches(mark)
    //     .lines()
    //     .next()
    //     .unwrap_or("")
    //     .trim();
    //
    // (position_offset, data.to_string())
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
