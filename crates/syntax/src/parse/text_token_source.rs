// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

//! See `TextTokenSource` docs.

use crate::parse::Token;

use crate::parse::lexer::RawToken;
use crate::{SyntaxKind::EOF, TextRange, TextSize};

/// Implementation of `parser::TokenSource` that takes tokens from source code text.
#[derive(Debug)]
pub(crate) struct TextTokenSource {
    text: String,
    /// token and its start position (non-whitespace/comment tokens)
    /// ```non-rust
    ///  struct Foo;
    ///  ^------^--^-
    ///  |      |    \________
    ///  |      \____         \
    ///  |           \         |
    ///  (struct, 0) (Foo, 7) (;, 10)
    /// ```
    /// `[(struct, 0), (Foo, 7), (;, 10)]`
    raw_tokens_with_offsets: Vec<(RawToken, TextSize)>,

    /// Current token and position
    curr: (Token, usize),
}

impl TextTokenSource {
    /// Generate input from tokens(expect comment and whitespace).
    pub(crate) fn new(text: &str, raw_tokens: Vec<RawToken>) -> TextTokenSource {
        let raw_tokens_with_offsets: Vec<_> = raw_tokens
            .iter()
            .filter_map({
                let mut offset = 0.into();
                move |token| {
                    // remove trivia from the token stream, preserving offsets
                    let token_with_offset = if token.kind.is_trivia() {
                        None
                    } else {
                        Some((*token, offset))
                    };
                    offset += token.len;
                    token_with_offset
                }
            })
            .collect();

        let first_token = token_at_pos(0, &raw_tokens_with_offsets);
        TextTokenSource {
            text: text.to_string(),
            raw_tokens_with_offsets,
            curr: (first_token, 0),
        }
    }

    pub(crate) fn current(&self) -> Token {
        self.curr.0
    }

    pub(crate) fn current_pos(&self) -> usize {
        self.curr.1
    }

    /// Lookahead n token
    pub(crate) fn lookahead_nth(&self, n: usize) -> Token {
        token_at_pos(self.curr.1 + n, &self.raw_tokens_with_offsets)
    }

    /// bump cursor to next token
    pub(crate) fn bump(&mut self) {
        if self.curr.0.kind == EOF {
            return;
        }

        let pos = self.curr.1 + 1;
        self.curr = (token_at_pos(pos, &self.raw_tokens_with_offsets), pos);
    }

    /// rollback to the previous token
    pub(crate) fn pop_position(&mut self) {
        let pos = self.curr.1 - 1;
        self.curr = (token_at_pos(pos, &self.raw_tokens_with_offsets), pos);
    }

    pub(crate) fn prev_text(&self) -> &str {
        self.raw_tokens_with_offsets
            .get(self.curr.1 - 1)
            .map(|(token, offset)| &self.text[TextRange::at(*offset, token.len)])
            .unwrap_or_default()
    }

    pub(crate) fn curr_pos(&self) -> usize {
        self.curr.1
    }

    pub(crate) fn token_range(&self, offset: usize) -> Option<TextRange> {
        // let curr_pos = self.curr.1 as isize;
        // if curr_pos < offset {
        //     return None;
        // }
        self.raw_tokens_with_offsets
            .get(offset)
            .map(|(token, offset)| TextRange::at(*offset, token.len))
    }

    pub(crate) fn current_text(&self) -> &str {
        self.raw_tokens_with_offsets
            .get(self.curr.1)
            .map(|(token, offset)| &self.text[TextRange::at(*offset, token.len)])
            .unwrap_or_default()
    }

    pub(crate) fn next_text(&self) -> &str {
        self.raw_tokens_with_offsets
            .get(self.curr.1 + 1)
            .map(|(token, offset)| &self.text[TextRange::at(*offset, token.len)])
            .unwrap_or_default()
    }

    /// Is the current token a specified keyword?
    pub(crate) fn is_keyword(&self, kw: &str) -> bool {
        self.raw_tokens_with_offsets
            .get(self.curr.1)
            .map_or(false, |(token, offset)| {
                &self.text[TextRange::at(*offset, token.len)] == kw
            })
    }
}

fn token_at_pos(pos: usize, tokens_with_offsets: &[(RawToken, TextSize)]) -> Token {
    let (kind, is_jointed_to_next) = match tokens_with_offsets.get(pos) {
        Some((raw_token, offset)) => (
            raw_token.kind,
            tokens_with_offsets
                .get(pos + 1)
                .map_or(false, |(_, next_offset)| offset + raw_token.len == *next_offset),
        ),
        None => (EOF, false),
    };
    Token { kind, is_jointed_to_next }
}
