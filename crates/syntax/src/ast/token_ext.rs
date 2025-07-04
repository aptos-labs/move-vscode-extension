// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::{AstToken, TextRange, TextSize, ast};
use stdx::always;

impl ast::Comment {
    pub fn kind(&self) -> CommentKind {
        CommentKind::from_text(self.text())
    }

    pub fn is_doc(&self) -> bool {
        self.kind().doc.is_some()
    }

    pub fn is_inner(&self) -> bool {
        self.kind().doc == Some(CommentPlacement::Inner)
    }

    pub fn is_outer(&self) -> bool {
        self.kind().doc == Some(CommentPlacement::Outer)
    }

    pub fn prefix(&self) -> &'static str {
        let &(prefix, _kind) = CommentKind::BY_PREFIX
            .iter()
            .find(|&(prefix, kind)| self.kind() == *kind && self.text().starts_with(prefix))
            .unwrap();
        prefix
    }

    /// Returns the textual content of a doc comment node as a single string with prefix and suffix
    /// removed.
    pub fn comment_line(&self) -> Option<&str> {
        let kind = self.kind();
        match kind {
            CommentKind { shape, doc: Some(_) } => {
                let prefix = kind.prefix();
                let text = &self.text()[prefix.len()..];
                let text = if shape == CommentShape::Block {
                    text.strip_suffix("*/").unwrap_or(text)
                } else {
                    text
                };
                Some(text)
            }
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct CommentKind {
    pub shape: CommentShape,
    pub doc: Option<CommentPlacement>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CommentShape {
    Line,
    Block,
}

impl CommentShape {
    pub fn is_line(self) -> bool {
        self == CommentShape::Line
    }

    pub fn is_block(self) -> bool {
        self == CommentShape::Block
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CommentPlacement {
    Inner,
    Outer,
}

impl CommentKind {
    const BY_PREFIX: [(&'static str, CommentKind); 9] = [
        (
            "/**/",
            CommentKind {
                shape: CommentShape::Block,
                doc: None,
            },
        ),
        (
            "/***",
            CommentKind {
                shape: CommentShape::Block,
                doc: None,
            },
        ),
        (
            "////",
            CommentKind {
                shape: CommentShape::Line,
                doc: None,
            },
        ),
        (
            "///",
            CommentKind {
                shape: CommentShape::Line,
                doc: Some(CommentPlacement::Outer),
            },
        ),
        (
            "//!",
            CommentKind {
                shape: CommentShape::Line,
                doc: Some(CommentPlacement::Inner),
            },
        ),
        (
            "/**",
            CommentKind {
                shape: CommentShape::Block,
                doc: Some(CommentPlacement::Outer),
            },
        ),
        (
            "/*!",
            CommentKind {
                shape: CommentShape::Block,
                doc: Some(CommentPlacement::Inner),
            },
        ),
        (
            "//",
            CommentKind {
                shape: CommentShape::Line,
                doc: None,
            },
        ),
        (
            "/*",
            CommentKind {
                shape: CommentShape::Block,
                doc: None,
            },
        ),
    ];

    pub(crate) fn from_text(text: &str) -> CommentKind {
        let &(_prefix, kind) = CommentKind::BY_PREFIX
            .iter()
            .find(|&(prefix, _kind)| text.starts_with(prefix))
            .unwrap();
        kind
    }

    pub fn prefix(&self) -> &'static str {
        let &(prefix, _) = CommentKind::BY_PREFIX
            .iter()
            .rev()
            .find(|(_, kind)| kind == self)
            .unwrap();
        prefix
    }
}

impl ast::Whitespace {
    pub fn spans_multiple_lines(&self) -> bool {
        let text = self.text();
        text.find('\n').is_some_and(|idx| text[idx + 1..].contains('\n'))
    }
}

#[derive(Debug)]
pub struct QuoteOffsets {
    pub quotes: (TextRange, TextRange),
    pub contents: TextRange,
}

impl QuoteOffsets {
    fn new(literal: &str, left_quote: &str) -> Option<QuoteOffsets> {
        let left_quote_idx = literal.find(left_quote)?;
        let right_quote_idx = literal.rfind('"')?;
        if left_quote_idx == right_quote_idx - (left_quote.len() - 1) {
            // `literal` only contains one quote
            return None;
        }

        let start = TextSize::from(0);
        let left_quote = TextSize::try_from(left_quote_idx).unwrap() + TextSize::of(left_quote);
        let right_quote = TextSize::try_from(right_quote_idx).unwrap();
        let end = TextSize::of(literal);

        let res = QuoteOffsets {
            quotes: (
                TextRange::new(start, left_quote),
                TextRange::new(right_quote, end),
            ),
            contents: TextRange::new(left_quote, right_quote),
        };
        Some(res)
    }
}

pub trait IsString: AstToken {
    const LEFT_QUOTE: &'static str;

    fn quote_offsets(&self) -> Option<QuoteOffsets> {
        let text = self.text();
        let offsets = QuoteOffsets::new(text, Self::LEFT_QUOTE)?;
        let o = self.syntax().text_range().start();
        let offsets = QuoteOffsets {
            quotes: (offsets.quotes.0 + o, offsets.quotes.1 + o),
            contents: offsets.contents + o,
        };
        Some(offsets)
    }

    fn text_range_between_quotes(&self) -> Option<TextRange> {
        self.quote_offsets().map(|it| it.contents)
    }

    fn text_without_quotes(&self) -> &str {
        let text = self.text();
        let Some(offsets) = self.text_range_between_quotes() else {
            return text;
        };
        &text[offsets - self.syntax().text_range().start()]
    }

    fn open_quote_text_range(&self) -> Option<TextRange> {
        self.quote_offsets().map(|it| it.quotes.0)
    }

    fn close_quote_text_range(&self) -> Option<TextRange> {
        self.quote_offsets().map(|it| it.quotes.1)
    }

    fn map_range_up(&self, range: TextRange) -> Option<TextRange> {
        let contents_range = self.text_range_between_quotes()?;
        if always!(TextRange::up_to(contents_range.len()).contains_range(range)) {
            Some(range + contents_range.start())
        } else {
            None
        }
    }
}

impl IsString for ast::ByteString {
    const LEFT_QUOTE: &'static str = "b\"";
}

impl IsString for ast::HexString {
    const LEFT_QUOTE: &'static str = "x\"";
}
