//! Representation of a `TextEdit`.
//!
//! `rust-analyzer` never mutates text itself and only sends diffs to clients,
//! so `TextEdit` is the ultimate representation of the work done by
//! rust-analyzer.

use itertools::Itertools;
use std::cmp::max;
use syntax::{TextRange, TextSize};

/// `ReplaceText` -- a single "atomic" change to text
///
/// Must not overlap with other `ReplaceText`s
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextChange {
    pub new_text: String,
    /// Refers to offsets in the original text
    pub range: TextRange,
}

#[derive(Default, Debug, Clone)]
pub struct TextEdit {
    /// Invariant: disjoint and sorted by `range`.
    changes: Vec<TextChange>,
}

#[derive(Debug, Default, Clone)]
pub struct TextEditBuilder {
    text_changes: Vec<TextChange>,
}

impl TextChange {
    pub fn insert(offset: TextSize, text: String) -> TextChange {
        TextChange::replace(TextRange::empty(offset), text)
    }
    pub fn delete(range: TextRange) -> TextChange {
        TextChange::replace(range, String::new())
    }
    pub fn replace(range: TextRange, replace_with: String) -> TextChange {
        TextChange {
            range,
            new_text: replace_with,
        }
    }
    pub fn apply(&self, text: &mut String) {
        let start: usize = self.range.start().into();
        let end: usize = self.range.end().into();
        text.replace_range(start..end, &self.new_text);
    }
}

impl TextEdit {
    pub fn builder() -> TextEditBuilder {
        TextEditBuilder::default()
    }

    pub fn insert(offset: TextSize, text: String) -> TextEdit {
        let mut builder = TextEdit::builder();
        builder.insert(offset, text);
        builder.finish()
    }

    pub fn delete(range: TextRange) -> TextEdit {
        let mut builder = TextEdit::builder();
        builder.delete(range);
        builder.finish()
    }

    pub fn replace(range: TextRange, replace_with: String) -> TextEdit {
        let mut builder = TextEdit::builder();
        builder.replace(range, replace_with);
        builder.finish()
    }

    pub fn len(&self) -> usize {
        self.changes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, TextChange> {
        self.into_iter()
    }

    pub fn apply(&self, text: &mut String) {
        match self.len() {
            0 => return,
            1 => {
                self.changes[0].apply(text);
                return;
            }
            _ => (),
        }

        // optimization
        let original_len = TextSize::of(&*text);
        let mut total_len = original_len;
        let mut max_total_len = original_len;
        for text_change in &self.changes {
            total_len += TextSize::of(&text_change.new_text);
            total_len -= text_change.range.len();
            max_total_len = max(max_total_len, total_len);
        }
        if let Some(additional_len) = max_total_len.checked_sub(original_len) {
            text.reserve(additional_len.into());
        }

        for text_change in self.changes.iter().rev() {
            text_change.apply(text);
        }

        assert_eq!(TextSize::of(&*text), total_len);
    }

    pub fn union(&mut self, other: TextEdit) -> Result<(), TextEdit> {
        let iter_merge = self
            .iter()
            .merge_by(other.iter(), |l, r| l.range.start() <= r.range.start());
        if !check_disjoint(&mut iter_merge.clone()) {
            return Err(other);
        }

        // Only dedup deletions and replacements, keep all insertions
        self.changes = iter_merge
            .dedup_by(|a, b| a == b && !a.range.is_empty())
            .cloned()
            .collect();
        Ok(())
    }

    pub fn apply_to_offset(&self, offset: TextSize) -> Option<TextSize> {
        let mut res = offset;
        for indel in &self.changes {
            if indel.range.start() >= offset {
                break;
            }
            if offset < indel.range.end() {
                return None;
            }
            res += TextSize::of(&indel.new_text);
            res -= indel.range.len();
        }
        Some(res)
    }
}

impl IntoIterator for TextEdit {
    type Item = TextChange;
    type IntoIter = std::vec::IntoIter<TextChange>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.into_iter()
    }
}

impl<'a> IntoIterator for &'a TextEdit {
    type Item = &'a TextChange;
    type IntoIter = std::slice::Iter<'a, TextChange>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.iter()
    }
}

impl TextEditBuilder {
    pub fn is_empty(&self) -> bool {
        self.text_changes.is_empty()
    }
    pub fn replace(&mut self, range: TextRange, replace_with: String) {
        self.add_change(TextChange::replace(range, replace_with));
    }
    pub fn delete(&mut self, range: TextRange) {
        self.add_change(TextChange::delete(range));
    }
    pub fn insert(&mut self, offset: TextSize, text: String) {
        self.add_change(TextChange::insert(offset, text));
    }
    pub fn finish(self) -> TextEdit {
        let mut changes = self.text_changes;
        assert_disjoint_or_equal(&mut changes);
        changes = coalesce_text_changes(changes);
        TextEdit { changes }
    }
    pub fn invalidates_offset(&self, offset: TextSize) -> bool {
        self.text_changes
            .iter()
            .any(|indel| indel.range.contains_inclusive(offset))
    }
    fn add_change(&mut self, change: TextChange) {
        self.text_changes.push(change);
        if self.text_changes.len() <= 16 {
            assert_disjoint_or_equal(&mut self.text_changes);
        }
    }
}

fn assert_disjoint_or_equal(text_changes: &mut [TextChange]) {
    assert!(check_disjoint_and_sort(text_changes));
}

fn check_disjoint_and_sort(text_changes: &mut [TextChange]) -> bool {
    text_changes.sort_by_key(|change| (change.range.start(), change.range.end()));
    check_disjoint(&mut text_changes.iter())
}

fn check_disjoint<'a, I>(text_changes: &mut I) -> bool
where
    I: Iterator<Item = &'a TextChange> + Clone,
{
    text_changes
        .clone()
        .zip(text_changes.skip(1))
        .all(|(l, r)| l.range.end() <= r.range.start() || l == r)
}

fn coalesce_text_changes(indels: Vec<TextChange>) -> Vec<TextChange> {
    indels
        .into_iter()
        .coalesce(|mut a, b| {
            if a.range.end() == b.range.start() {
                a.new_text.push_str(&b.new_text);
                a.range = TextRange::new(a.range.start(), b.range.end());
                Ok(a)
            } else {
                Err((a, b))
            }
        })
        .collect_vec()
}

#[cfg(test)]
mod tests {
    use super::{TextEdit, TextEditBuilder, TextRange};

    fn range(start: u32, end: u32) -> TextRange {
        TextRange::new(start.into(), end.into())
    }

    #[test]
    fn test_apply() {
        let mut text = "_11h1_2222_xx3333_4444_6666".to_owned();
        let mut builder = TextEditBuilder::default();
        builder.replace(range(3, 4), "1".to_owned());
        builder.delete(range(11, 13));
        builder.insert(22.into(), "_5555".to_owned());

        let text_edit = builder.finish();
        text_edit.apply(&mut text);

        assert_eq!(text, "_1111_2222_3333_4444_5555_6666")
    }

    #[test]
    fn test_union() {
        let mut edit1 = TextEdit::delete(range(7, 11));
        let mut builder = TextEditBuilder::default();
        builder.delete(range(1, 5));
        builder.delete(range(13, 17));

        let edit2 = builder.finish();
        assert!(edit1.union(edit2).is_ok());
        assert_eq!(edit1.changes.len(), 3);
    }

    #[test]
    fn test_union_with_duplicates() {
        let mut builder1 = TextEditBuilder::default();
        builder1.delete(range(7, 11));
        builder1.delete(range(13, 17));

        let mut builder2 = TextEditBuilder::default();
        builder2.delete(range(1, 5));
        builder2.delete(range(13, 17));

        let mut edit1 = builder1.finish();
        let edit2 = builder2.finish();
        assert!(edit1.union(edit2).is_ok());
        assert_eq!(edit1.changes.len(), 3);
    }

    #[test]
    fn test_union_panics() {
        let mut edit1 = TextEdit::delete(range(7, 11));
        let edit2 = TextEdit::delete(range(9, 13));
        assert!(edit1.union(edit2).is_err());
    }

    #[test]
    fn test_coalesce_disjoint() {
        let mut builder = TextEditBuilder::default();
        builder.replace(range(1, 3), "aa".into());
        builder.replace(range(5, 7), "bb".into());
        let edit = builder.finish();

        assert_eq!(edit.changes.len(), 2);
    }

    #[test]
    fn test_coalesce_adjacent() {
        let mut builder = TextEditBuilder::default();
        builder.replace(range(1, 3), "aa".into());
        builder.replace(range(3, 5), "bb".into());

        let edit = builder.finish();
        assert_eq!(edit.changes.len(), 1);
        assert_eq!(edit.changes[0].new_text, "aabb");
        assert_eq!(edit.changes[0].range, range(1, 5));
    }

    #[test]
    fn test_coalesce_adjacent_series() {
        let mut builder = TextEditBuilder::default();
        builder.replace(range(1, 3), "au".into());
        builder.replace(range(3, 5), "www".into());
        builder.replace(range(5, 8), "".into());
        builder.replace(range(8, 9), "ub".into());

        let edit = builder.finish();
        assert_eq!(edit.changes.len(), 1);
        assert_eq!(edit.changes[0].new_text, "auwwwub");
        assert_eq!(edit.changes[0].range, range(1, 9));
    }
}
