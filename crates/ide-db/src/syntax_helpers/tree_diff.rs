//! Basic tree diffing functionality.
use rustc_hash::FxHashMap;
use syntax::{NodeOrToken, SyntaxElement, SyntaxNode};

use crate::{text_edit::TextEditBuilder, FxIndexMap};

#[derive(Debug, Hash, PartialEq, Eq)]
enum TreeDiffInsertPos {
    After(SyntaxElement),
    AsFirstChild(SyntaxElement),
}

#[derive(Debug)]
pub struct TreeDiff {
    replacements: FxHashMap<SyntaxElement, SyntaxElement>,
    deletions: Vec<SyntaxElement>,
    // the vec as well as the indexmap are both here to preserve order
    insertions: FxIndexMap<TreeDiffInsertPos, Vec<SyntaxElement>>,
}

impl TreeDiff {
    pub fn into_text_edit(&self, builder: &mut TextEditBuilder) {
        let _p = tracing::info_span!("into_text_edit").entered();

        for (anchor, to) in &self.insertions {
            let offset = match anchor {
                TreeDiffInsertPos::After(it) => it.text_range().end(),
                TreeDiffInsertPos::AsFirstChild(it) => it.text_range().start(),
            };
            to.iter().for_each(|to| builder.insert(offset, to.to_string()));
        }
        for (from, to) in &self.replacements {
            builder.replace(from.text_range(), to.to_string());
        }
        for text_range in self.deletions.iter().map(SyntaxElement::text_range) {
            builder.delete(text_range);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.replacements.is_empty() && self.deletions.is_empty() && self.insertions.is_empty()
    }
}

/// Finds a (potentially minimal) diff, which, applied to `from`, will result in `to`.
///
/// Specifically, returns a structure that consists of a replacements, insertions and deletions
/// such that applying this map on `from` will result in `to`.
///
/// This function tries to find a fine-grained diff.
pub fn diff(from: &SyntaxNode, to: &SyntaxNode) -> TreeDiff {
    let _p = tracing::info_span!("diff").entered();

    let mut diff = TreeDiff {
        replacements: FxHashMap::default(),
        insertions: FxIndexMap::default(),
        deletions: Vec::new(),
    };
    let (from, to) = (from.clone().into(), to.clone().into());

    if !syntax_element_eq(&from, &to) {
        go(&mut diff, from, to);
    }
    return diff;

    fn syntax_element_eq(lhs: &SyntaxElement, rhs: &SyntaxElement) -> bool {
        lhs.kind() == rhs.kind()
            && lhs.text_range().len() == rhs.text_range().len()
            && match (&lhs, &rhs) {
                (NodeOrToken::Node(lhs), NodeOrToken::Node(rhs)) => {
                    lhs == rhs || lhs.text() == rhs.text()
                }
                (NodeOrToken::Token(lhs), NodeOrToken::Token(rhs)) => lhs.text() == rhs.text(),
                _ => false,
            }
    }

    // FIXME: this is horribly inefficient. I bet there's a cool algorithm to diff trees properly.
    fn go(diff: &mut TreeDiff, lhs: SyntaxElement, rhs: SyntaxElement) {
        let (lhs, rhs) = match lhs.as_node().zip(rhs.as_node()) {
            Some((lhs, rhs)) => (lhs, rhs),
            _ => {
                diff.replacements.insert(lhs, rhs);
                return;
            }
        };

        let mut look_ahead_scratch = Vec::default();

        let mut rhs_children = rhs.children_with_tokens();
        let mut lhs_children = lhs.children_with_tokens();
        let mut last_lhs = None;
        loop {
            let lhs_child = lhs_children.next();
            match (lhs_child.clone(), rhs_children.next()) {
                (None, None) => break,
                (None, Some(element)) => {
                    let insert_pos = match last_lhs.clone() {
                        Some(prev) => TreeDiffInsertPos::After(prev),
                        // first iteration, insert into out parent as the first child
                        None => TreeDiffInsertPos::AsFirstChild(lhs.clone().into()),
                    };
                    diff.insertions.entry(insert_pos).or_default().push(element);
                }
                (Some(element), None) => {
                    diff.deletions.push(element);
                }
                (Some(ref lhs_ele), Some(ref rhs_ele)) if syntax_element_eq(lhs_ele, rhs_ele) => {}
                (Some(lhs_ele), Some(rhs_ele)) => {
                    // nodes differ, look for lhs_ele in rhs, if its found we can mark everything up
                    // until that element as insertions. This is important to keep the diff minimal
                    // in regards to insertions that have been actually done, this is important for
                    // use insertions as we do not want to replace the entire module node.
                    look_ahead_scratch.push(rhs_ele.clone());
                    let mut rhs_children_clone = rhs_children.clone();
                    let mut insert = false;
                    for rhs_child in &mut rhs_children_clone {
                        if syntax_element_eq(&lhs_ele, &rhs_child) {
                            insert = true;
                            break;
                        }
                        look_ahead_scratch.push(rhs_child);
                    }
                    let drain = look_ahead_scratch.drain(..);
                    if insert {
                        let insert_pos = if let Some(prev) = last_lhs.clone().filter(|_| insert) {
                            TreeDiffInsertPos::After(prev)
                        } else {
                            TreeDiffInsertPos::AsFirstChild(lhs.clone().into())
                        };

                        diff.insertions.entry(insert_pos).or_default().extend(drain);
                        rhs_children = rhs_children_clone;
                    } else {
                        go(diff, lhs_ele, rhs_ele);
                    }
                }
            }
            last_lhs = lhs_child.or(last_lhs);
        }
    }
}

#[cfg(test)]
mod tests {
    // todo: add tests, see rust-analyzer
}
