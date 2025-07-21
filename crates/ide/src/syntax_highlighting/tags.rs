// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use ide_db::SymbolKind;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Highlight {
    pub tag: HlTag,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HlPunct {
    /// []
    Bracket,
    /// {}
    Brace,
    /// ()
    Parenthesis,
    /// <>
    Angle,
    /// ,
    Comma,
    /// .
    Dot,
    /// :
    Colon,
    /// ;
    Semi,
    /// ! (only for macro calls)
    MacroBang,
    /// Other punctutations
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HlOperator {
    /// |, &, !, ^, |=, &=, ^=
    Bitwise,
    /// +, -, *, /, +=, -=, *=, /=
    Arithmetic,
    /// &&, ||, !
    Logical,
    /// >, <, ==, >=, <=, !=
    Comparison,
    /// Other operators
    Other,
}

impl From<HlTag> for Highlight {
    fn from(tag: HlTag) -> Highlight {
        Highlight::new(tag)
    }
}

impl From<HlOperator> for Highlight {
    fn from(op: HlOperator) -> Highlight {
        Highlight::new(HlTag::Operator(op))
    }
}

impl From<HlPunct> for Highlight {
    fn from(punct: HlPunct) -> Highlight {
        Highlight::new(HlTag::Punctuation(punct))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HlTag {
    Symbol(SymbolKind),

    AttributeBracket,
    BoolLiteral,
    BuiltinType,
    Comment,
    Keyword,
    NumericLiteral,
    Operator(HlOperator),
    Punctuation(HlPunct),
    StringLiteral,
    UnresolvedReference,

    // For things which don't have a specific highlight.
    None,
}

impl HlTag {
    fn as_str(self) -> &'static str {
        match self {
            HlTag::Symbol(symbol) => match symbol {
                SymbolKind::Attribute => "attribute",
                SymbolKind::Const => "constant",
                SymbolKind::Enum => "enum",
                SymbolKind::Field => "field",
                SymbolKind::Function => "function",
                SymbolKind::Label => "label",
                SymbolKind::Local => "variable",
                // SymbolKind::Macro => "macro",
                SymbolKind::Method => "method",
                SymbolKind::Module => "module",
                SymbolKind::Struct => "struct",
                SymbolKind::TypeParam => "type_param",
                SymbolKind::ValueParam => "value_param",
                SymbolKind::EnumVariant => "enum_variant",
                SymbolKind::GlobalVariableDecl => "global",
                SymbolKind::Vector => "vector",
                SymbolKind::Assert => "assert",
                SymbolKind::Schema => "schema",
            },
            HlTag::AttributeBracket => "attribute_bracket",
            HlTag::BoolLiteral => "bool_literal",
            HlTag::BuiltinType => "builtin_type",
            HlTag::Comment => "comment",
            HlTag::Keyword => "keyword",
            HlTag::Punctuation(punct) => match punct {
                HlPunct::Bracket => "bracket",
                HlPunct::Brace => "brace",
                HlPunct::Parenthesis => "parenthesis",
                HlPunct::Angle => "angle",
                HlPunct::Comma => "comma",
                HlPunct::Dot => "dot",
                HlPunct::Colon => "colon",
                HlPunct::Semi => "semicolon",
                HlPunct::MacroBang => "macro_bang",
                HlPunct::Other => "punctuation",
            },
            HlTag::NumericLiteral => "numeric_literal",
            HlTag::Operator(op) => match op {
                HlOperator::Bitwise => "bitwise",
                HlOperator::Arithmetic => "arithmetic",
                HlOperator::Logical => "logical",
                HlOperator::Comparison => "comparison",
                HlOperator::Other => "operator",
            },
            HlTag::StringLiteral => "string_literal",
            HlTag::UnresolvedReference => "unresolved_reference",
            HlTag::None => "none",
        }
    }
}

impl fmt::Display for HlTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl fmt::Display for Highlight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.tag.fmt(f)
    }
}

impl Highlight {
    pub(crate) fn new(tag: HlTag) -> Highlight {
        Highlight { tag }
    }
    pub fn is_empty(&self) -> bool {
        self.tag == HlTag::None /*&& self.mods.is_empty()*/
    }
}
