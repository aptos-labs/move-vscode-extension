//! See `TextTokenSource` docs.

use parser::TokenSource;

use crate::{parsing::lexer::Token, SyntaxKind::EOF, TextRange, TextSize};

/// Implementation of `parser::TokenSource` that takes tokens from source code text.
pub(crate) struct TextTokenSource<'t> {
    text: &'t str,
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
    token_offset_pairs: Vec<(Token, TextSize)>,

    /// Current token and position
    curr: (parser::Token, usize),
}

impl<'t> TokenSource for TextTokenSource<'t> {
    fn current(&self) -> parser::Token {
        self.curr.0
    }

    fn lookahead_nth(&self, n: usize) -> parser::Token {
        mk_token(self.curr.1 + n, &self.token_offset_pairs)
    }

    fn bump(&mut self) {
        if self.curr.0.kind == EOF {
            return;
        }

        let pos = self.curr.1 + 1;
        self.curr = (mk_token(pos, &self.token_offset_pairs), pos);
    }

    fn rollback(&mut self) {
        let pos = self.curr.1 - 1;
        self.curr = (mk_token(pos, &self.token_offset_pairs), pos);
    }

    fn is_keyword(&self, kw: &str) -> bool {
        self.token_offset_pairs
            .get(self.curr.1)
            .map_or(false, |(token, offset)| {
                &self.text[TextRange::at(*offset, token.len)] == kw
            })
    }
}

fn mk_token(pos: usize, token_offset_pairs: &[(Token, TextSize)]) -> parser::Token {
    let (kind, is_jointed_to_next) = match token_offset_pairs.get(pos) {
        Some((token, offset)) => (
            token.kind,
            token_offset_pairs
                .get(pos + 1)
                .map_or(false, |(_, next_offset)| offset + token.len == *next_offset),
        ),
        None => (EOF, false),
    };
    parser::Token { kind, is_jointed_to_next }
}

impl<'t> TextTokenSource<'t> {
    /// Generate input from tokens(expect comment and whitespace).
    pub(crate) fn new(text: &'t str, raw_tokens: &'t [Token]) -> TextTokenSource<'t> {
        let token_offset_pairs: Vec<_> = raw_tokens
            .iter()
            .filter_map({
                let mut offset = 0.into();
                move |token| {
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

        let first = mk_token(0, &token_offset_pairs);
        TextTokenSource {
            text,
            token_offset_pairs,
            curr: (first, 0),
        }
    }
}
