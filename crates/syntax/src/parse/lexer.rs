// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::parse::SyntaxKind::*;
use crate::parse::lexer::move_model::Tok;
use crate::{SyntaxError, SyntaxKind, T, TextRange, TextSize};

mod move_model;

/// A token of Rust source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawToken {
    /// The kind of token.
    pub kind: SyntaxKind,
    /// The length of the token.
    pub len: TextSize,
}

/// Break a string up into its component tokens.
/// Beware that it checks for shebang first and its length contributes to resulting
/// tokens offsets.
pub fn tokenize(text: &str) -> (Vec<RawToken>, Vec<SyntaxError>) {
    // non-empty string is a precondition of `rustc_lexer::strip_shebang()`.
    if text.is_empty() {
        return Default::default();
    }

    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    let mut lex_tokens = move_model::Lexer::new(text);
    loop {
        lex_tokens.advance();
        if lex_tokens.peek() == Tok::EOF {
            break;
        }

        let syntax_kind = aptos_token_kind_to_syntax_kind(lex_tokens.peek(), lex_tokens.content());
        tokens.push(RawToken {
            kind: syntax_kind,
            len: TextSize::new(lex_tokens.content().len() as u32),
        });
    }

    (tokens, errors)
}

pub fn parse_single_token(text: &str) -> Option<RawToken> {
    let (mut tokens, _) = tokenize(text);
    if tokens.len() == 1 {
        return tokens.pop();
    }
    None
}

fn aptos_token_kind_to_syntax_kind(aptos_token_kind: Tok, token_text: &str) -> SyntaxKind {
    match aptos_token_kind {
        Tok::Whitespace => WHITESPACE,
        Tok::LineComment => COMMENT,
        Tok::BlockComment => COMMENT,

        Tok::BadCharacter => BAD_CHARACTER,

        Tok::NumValue => INT_NUMBER,
        Tok::NumTypedValue => INT_NUMBER,

        Tok::Identifier => IDENT,
        Tok::Label => QUOTE_IDENT,
        Tok::ByteStringValue => BYTE_STRING,
        Tok::HexStringValue => HEX_STRING,

        Tok::Plus => T![+],
        Tok::Minus => T![-],
        Tok::Star => T![*],
        Tok::Slash => T![/],
        Tok::Percent => T![%],

        Tok::AtSign => T![@],
        Tok::NumSign => T![#],
        Tok::Underscore => T!['_'],

        Tok::LParen => L_PAREN,
        Tok::RParen => R_PAREN,
        Tok::LBrace => L_CURLY,
        Tok::RBrace => R_CURLY,
        Tok::LBracket => L_BRACK,
        Tok::RBracket => R_BRACK,

        Tok::Less => T![<],
        Tok::Greater => T![>],
        // Tok::GreaterEqual => T![>=],
        // Tok::LessEqual => T![<=],
        Tok::EqualEqual => T![==],
        Tok::ExclaimEqual => T![!=],

        // Tok::LessLess => T![<<],
        // Tok::GreaterGreater => T![>>],
        Tok::Caret => T![^],
        Tok::Amp => T![&],
        Tok::Pipe => T![|],

        // Tok::PipePipe => T![||],
        // Tok::AmpAmp => T![&&],
        Tok::PlusEqual => T![+=],
        Tok::SubEqual => T![-=],
        Tok::MulEqual => T![*=],
        Tok::DivEqual => T![/=],
        Tok::ModEqual => T![%=],

        Tok::XorEqual => T![^=],
        Tok::BitAndEqual => T![&=],
        Tok::BitOrEqual => T![|=],

        Tok::EqualGreater => T![=>],
        Tok::EqualEqualGreater => T![==>],
        // Tok::ShlEqual => T![<<=],
        // Tok::ShrEqual => T![>>=],
        Tok::As => T![as],
        Tok::Use => T![use],
        Tok::Break => T![break],
        Tok::Continue => T![continue],
        Tok::If => T![if],
        Tok::Else => T![else],
        Tok::While => T![while],
        Tok::Mut => T![mut],
        Tok::Loop => T![loop],
        Tok::Abort => T![abort],
        Tok::Return => T![return],
        Tok::True => T![true],
        Tok::False => T![false],
        Tok::Let => T![let],
        Tok::Struct => T![struct],
        Tok::Fun => T![fun],
        Tok::Const => T![const],
        Tok::Module => T![module],
        Tok::Script => T![script],
        Tok::Spec => T![spec],
        Tok::Invariant => T![invariant],
        Tok::Acquires => T![acquires],
        Tok::Friend => T![friend],
        Tok::Inline => T![inline],

        Tok::Public => T![public],
        Tok::Native => T![native],

        // Tok::Copy => T![copy],
        Tok::Move => T![move],

        Tok::Equal => T![=],
        Tok::Amp => T![&],
        Tok::Pipe => T![|],
        Tok::Exclaim => T![!],

        Tok::Period => T![.],
        Tok::Comma => T![,],
        Tok::Semicolon => T![;],
        Tok::Colon => T![:],
        Tok::ColonColon => T![::],

        Tok::PeriodPeriod => T![..],

        _ => unimplemented!("for {:?}", aptos_token_kind),
    }
}
