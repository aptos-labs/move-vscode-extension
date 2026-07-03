// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

//! Semantic Tokens helpers

use lsp_types::{Range, SemanticToken, SemanticTokenTypes, SemanticTokens, SemanticTokensEdit};
use std::fmt;
use std::slice::Iter;

macro_rules! declare_enum {
    (
        $(#[$attrs:meta])*
        $visibility:vis enum $name:ident {
            $($variant:ident),* $(,)?
        }
    ) => {
        $(#[$attrs])*
        $visibility enum $name {
            $($variant,)*
        }

        impl $name {
            pub(crate) fn iter() -> Iter<'static, Self> {
                static ITEMS: &[$name] = &[
                    $(
                        $name::$variant,
                    )*
                ];
                ITEMS.iter()
            }
        }
    };
}

macro_rules! define_semantic_token_types {
    (
        standard {
            $($standard:ident),*$(,)?
        }
        custom {
            $(($custom:ident, $string:literal) $(=> $fallback:ident)?),*$(,)?
        }

    ) => {
        pub(crate) mod types {
            use super::SemanticTokenTypes;
            $(pub(crate) const $standard: SemanticTokenTypes = SemanticTokenTypes::$standard;)*
            $(pub(crate) const $custom: SemanticTokenTypes = SemanticTokenTypes::new($string);)*
        }

        pub(crate) const SUPPORTED_TYPES: &[SemanticTokenTypes] = &[
            $(SemanticTokenTypes::$standard,)*
            $(self::types::$custom),*
        ];

        pub(crate) fn standard_fallback_type(token: SemanticTokenTypes) -> Option<SemanticTokenTypes> {
            use self::types::*;
            $(
                if token == $custom {
                    None $(.or(Some(SemanticTokenTypes::$fallback)))?
                } else
            )*
            { Some(token )}
        }
    };
}

declare_enum! {
    #[repr(u32)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub(crate) enum SupportedType {
        Comment,
        Decorator,
        EnumMember,
        Enum,
        Function,
        Interface,
        Keyword,
        Macro,
        Method,
        Namespace,
        Number,
        Operator,
        Parameter,
        Property,
        String,
        Struct,
        TypeParameter,
        Variable,
        Type,
        Label,
        Angle,
        Arithmetic,
        AttributeBracket,
        Attribute,
        Bitwise,
        Boolean,
        Brace,
        Bracket,
        BuiltinAttribute,
        BuiltinType,
        Char,
        Colon,
        Comma,
        Comparison,
        ConstParameter,
        Const,
        DeriveHelper,
        Derive,
        Dot,
        EscapeSequence,
        FormatSpecifier,
        Generic,
        InvalidEscapeSequence,
        Lifetime,
        Logical,
        MacroBang,
        Negation,
        Parenthesis,
        ProcMacro,
        Punctuation,
        SelfKeyword,
        SelfTypeKeyword,
        Semicolon,
        Static,
        ToolModule,
        TypeAlias,
        Union,
        UnresolvedReference,
    }
}

impl fmt::Display for SupportedType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = match self {
            SupportedType::Comment => SemanticTokenTypes::Comment.as_str(),
            SupportedType::Decorator => SemanticTokenTypes::Decorator.as_str(),
            SupportedType::EnumMember => SemanticTokenTypes::EnumMember.as_str(),
            SupportedType::Enum => SemanticTokenTypes::Enum.as_str(),
            SupportedType::Function => SemanticTokenTypes::Function.as_str(),
            SupportedType::Interface => SemanticTokenTypes::Interface.as_str(),
            SupportedType::Keyword => SemanticTokenTypes::Keyword.as_str(),
            SupportedType::Macro => SemanticTokenTypes::Macro.as_str(),
            SupportedType::Method => SemanticTokenTypes::Method.as_str(),
            SupportedType::Namespace => SemanticTokenTypes::Namespace.as_str(),
            SupportedType::Number => SemanticTokenTypes::Number.as_str(),
            SupportedType::Operator => SemanticTokenTypes::Operator.as_str(),
            SupportedType::Parameter => SemanticTokenTypes::Parameter.as_str(),
            SupportedType::Property => SemanticTokenTypes::Property.as_str(),
            SupportedType::String => SemanticTokenTypes::String.as_str(),
            SupportedType::Struct => SemanticTokenTypes::Struct.as_str(),
            SupportedType::TypeParameter => SemanticTokenTypes::TypeParameter.as_str(),
            SupportedType::Variable => SemanticTokenTypes::Variable.as_str(),
            SupportedType::Type => SemanticTokenTypes::Type.as_str(),
            SupportedType::Label => SemanticTokenTypes::Label.as_str(),
            SupportedType::Angle => "angle",
            SupportedType::Arithmetic => "arithmetic",
            SupportedType::AttributeBracket => "attributeBracket",
            SupportedType::Attribute => "attribute",
            SupportedType::Bitwise => "bitwise",
            SupportedType::Boolean => "boolean",
            SupportedType::Brace => "brace",
            SupportedType::Bracket => "bracket",
            SupportedType::BuiltinAttribute => "builtinAttribute",
            SupportedType::BuiltinType => "builtinType",
            SupportedType::Char => "character",
            SupportedType::Colon => "colon",
            SupportedType::Comma => "comma",
            SupportedType::Comparison => "comparison",
            SupportedType::ConstParameter => "constParameter",
            SupportedType::Const => "const",
            SupportedType::DeriveHelper => "deriveHelper",
            SupportedType::Derive => "derive",
            SupportedType::Dot => "dot",
            SupportedType::EscapeSequence => "escapeSequence",
            SupportedType::FormatSpecifier => "formatSpecifier",
            SupportedType::Generic => "generic",
            SupportedType::InvalidEscapeSequence => "invalidEscapeSequence",
            SupportedType::Lifetime => "lifetime",
            SupportedType::Logical => "logical",
            SupportedType::MacroBang => "macroBang",
            SupportedType::Negation => "negation",
            SupportedType::Parenthesis => "parenthesis",
            SupportedType::ProcMacro => "procMacro",
            SupportedType::Punctuation => "punctuation",
            SupportedType::SelfKeyword => "selfKeyword",
            SupportedType::SelfTypeKeyword => "selfTypeKeyword",
            SupportedType::Semicolon => "semicolon",
            SupportedType::Static => "static",
            SupportedType::ToolModule => "toolModule",
            SupportedType::TypeAlias => "typeAlias",
            SupportedType::Union => "union",
            SupportedType::UnresolvedReference => "unresolvedReference",
        };
        f.write_str(string)
    }
}

pub(crate) fn standard_fallback_type(token: SupportedType) -> Option<SupportedType> {
    Some(match token {
        SupportedType::Comment => SupportedType::Comment,
        SupportedType::Decorator => SupportedType::Decorator,
        SupportedType::EnumMember => SupportedType::EnumMember,
        SupportedType::Enum => SupportedType::Enum,
        SupportedType::Function => SupportedType::Function,
        SupportedType::Interface => SupportedType::Interface,
        SupportedType::Keyword => SupportedType::Keyword,
        SupportedType::Macro => SupportedType::Macro,
        SupportedType::Method => SupportedType::Method,
        SupportedType::Namespace => SupportedType::Namespace,
        SupportedType::Number => SupportedType::Number,
        SupportedType::Operator => SupportedType::Operator,
        SupportedType::Parameter => SupportedType::Parameter,
        SupportedType::Property => SupportedType::Property,
        SupportedType::String => SupportedType::String,
        SupportedType::Struct => SupportedType::Struct,
        SupportedType::TypeParameter => SupportedType::TypeParameter,
        SupportedType::Variable => SupportedType::Variable,
        SupportedType::Type => SupportedType::Type,
        SupportedType::Label => SupportedType::Label,
        _ => return None,
    })
}

// define_semantic_token_types![
//     standard {
//         COMMENT,
//         DECORATOR,
//         ENUM_MEMBER,
//         ENUM,
//         FUNCTION,
//         KEYWORD,
//         METHOD,
//         NAMESPACE,
//         NUMBER,
//         OPERATOR,
//         PARAMETER,
//         PROPERTY,
//         STRING,
//         STRUCT,
//         TYPE_PARAMETER,
//         VARIABLE,
//         MACRO,
//     }
//
//     custom {
//         (ANGLE, "angle"),
//         (ARITHMETIC, "arithmetic") => OPERATOR,
//         (ATTRIBUTE, "attribute") => DECORATOR,
//         (ATTRIBUTE_BRACKET, "attributeBracket") => DECORATOR,
//         (BITWISE, "bitwise") => OPERATOR,
//         (BOOLEAN, "boolean"),
//         (BRACE, "brace"),
//         (BRACKET, "bracket"),
//         (BUILTIN_TYPE, "builtinType") => TYPE,
//         (COLON, "colon"),
//         (COMMA, "comma"),
//         (COMPARISON, "comparison") => OPERATOR,
//         (CONST, "const") => VARIABLE,
//         (DOT, "dot"),
//         (GENERIC, "generic") => TYPE_PARAMETER,
//         (LABEL, "label"),
//         (LOGICAL, "logical") => OPERATOR,
//         (MACRO_BANG, "macroBang") => MACRO,
//         (PARENTHESIS, "parenthesis"),
//         (PUNCTUATION, "punctuation"),
//         (SEMICOLON, "semicolon"),
//         (UNRESOLVED_REFERENCE, "unresolvedReference"),
//     }
// ];

// macro_rules! count_tts {
//     () => {0usize};
//     ($_head:tt $($tail:tt)*) => {1usize + count_tts!($($tail)*)};
// }

// define_semantic_token_modifiers![
//     standard {
//         ASYNC,
//         DOCUMENTATION,
//         DECLARATION,
//         STATIC,
//         DEFAULT_LIBRARY,
//     }
//     custom {
//         (ASSOCIATED, "associated"),
//         (ATTRIBUTE_MODIFIER, "attribute"),
//         (CALLABLE, "callable"),
//         (CONSTANT, "constant"),
//         (CONSUMING, "consuming"),
//         (CONTROL_FLOW, "controlFlow"),
//         (CRATE_ROOT, "crateRoot"),
//         (INJECTED, "injected"),
//         (INTRA_DOC_LINK, "intraDocLink"),
//         (LIBRARY, "library"),
//         (MACRO_MODIFIER, "macro"),
//         (MUTABLE, "mutable"),
//         (PROC_MACRO_MODIFIER, "procMacro"),
//         (PUBLIC, "public"),
//         (REFERENCE, "reference"),
//         (TRAIT_MODIFIER, "trait"),
//         (UNSAFE, "unsafe"),
//     }
// ];

#[derive(Default)]
pub(crate) struct ModifierSet(pub(crate) u32);

// impl ModifierSet {
//     pub(crate) fn standard_fallback(&mut self) {
//         // Remove all non standard modifiers
//         self.0 &= !(!0u32 << LAST_STANDARD_MOD)
//     }
// }
//
// impl ops::BitOrAssign<SemanticTokenModifier> for ModifierSet {
//     fn bitor_assign(&mut self, rhs: SemanticTokenModifier) {
//         let idx = SUPPORTED_MODIFIERS.iter().position(|it| it == &rhs).unwrap();
//         self.0 |= 1 << idx;
//     }
// }

/// Tokens are encoded relative to each other.
///
/// This is a direct port of <https://github.com/microsoft/vscode-languageserver-node/blob/f425af9de46a0187adb78ec8a46b9b2ce80c5412/server/src/sematicTokens.proposed.ts#L45>
pub(crate) struct SemanticTokensBuilder {
    id: String,
    prev_line: u32,
    prev_char: u32,
    data: Vec<SemanticToken>,
}

impl SemanticTokensBuilder {
    pub(crate) fn new(id: String) -> Self {
        SemanticTokensBuilder {
            id,
            prev_line: 0,
            prev_char: 0,
            data: Vec::new(),
        }
    }

    /// Push a new token onto the builder
    pub(crate) fn push(&mut self, range: Range, token_index: u32) {
        let mut push_line = range.start.line;
        let mut push_char = range.start.character;

        if !self.data.is_empty() {
            push_line -= self.prev_line;
            if push_line == 0 {
                push_char -= self.prev_char;
            }
        }

        // A token cannot be multiline
        let token_len = range.end.character - range.start.character;

        let token = SemanticToken {
            delta_line: push_line,
            delta_start: push_char,
            length: token_len,
            token_type: token_index,
            token_modifiers_bitset: 0,
        };

        self.data.push(token);

        self.prev_line = range.start.line;
        self.prev_char = range.start.character;
    }

    pub(crate) fn build(self) -> SemanticTokens {
        SemanticTokens {
            result_id: Some(self.id),
            data: self.data,
        }
    }
}

pub(crate) fn diff_tokens(old: &[SemanticToken], new: &[SemanticToken]) -> Vec<SemanticTokensEdit> {
    let offset = new.iter().zip(old.iter()).take_while(|&(n, p)| n == p).count();

    let (_, old) = old.split_at(offset);
    let (_, new) = new.split_at(offset);

    let offset_from_end = new
        .iter()
        .rev()
        .zip(old.iter().rev())
        .take_while(|&(n, p)| n == p)
        .count();

    let (old, _) = old.split_at(old.len() - offset_from_end);
    let (new, _) = new.split_at(new.len() - offset_from_end);

    if old.is_empty() && new.is_empty() {
        vec![]
    } else {
        // The lsp data field is actually a byte-diff but we
        // travel in tokens so `start` and `delete_count` are in multiples of the
        // serialized size of `SemanticToken`.
        vec![SemanticTokensEdit {
            start: 5 * offset as u32,
            delete_count: 5 * old.len() as u32,
            data: Some(new.into()),
        }]
    }
}

pub(crate) fn type_index(kind: SupportedType) -> u32 {
    kind as u32
}
//
// pub(crate) fn type_index(ty: SemanticTokenTypes) -> u32 {
//     SUPPORTED_TYPES.iter().position(|it| *it == ty).unwrap() as u32
// }

#[cfg(test)]
mod tests {
    use super::*;

    fn from(t: (u32, u32, u32, u32, u32)) -> SemanticToken {
        SemanticToken {
            delta_line: t.0,
            delta_start: t.1,
            length: t.2,
            token_type: t.3,
            token_modifiers_bitset: t.4,
        }
    }

    #[test]
    fn test_diff_insert_at_end() {
        let before = [from((1, 2, 3, 4, 5)), from((6, 7, 8, 9, 10))];
        let after = [
            from((1, 2, 3, 4, 5)),
            from((6, 7, 8, 9, 10)),
            from((11, 12, 13, 14, 15)),
        ];

        let edits = diff_tokens(&before, &after);
        assert_eq!(
            edits[0],
            SemanticTokensEdit {
                start: 10,
                delete_count: 0,
                data: Some(vec![from((11, 12, 13, 14, 15))])
            }
        );
    }

    #[test]
    fn test_diff_insert_at_beginning() {
        let before = [from((1, 2, 3, 4, 5)), from((6, 7, 8, 9, 10))];
        let after = [
            from((11, 12, 13, 14, 15)),
            from((1, 2, 3, 4, 5)),
            from((6, 7, 8, 9, 10)),
        ];

        let edits = diff_tokens(&before, &after);
        assert_eq!(
            edits[0],
            SemanticTokensEdit {
                start: 0,
                delete_count: 0,
                data: Some(vec![from((11, 12, 13, 14, 15))])
            }
        );
    }

    #[test]
    fn test_diff_insert_in_middle() {
        let before = [from((1, 2, 3, 4, 5)), from((6, 7, 8, 9, 10))];
        let after = [
            from((1, 2, 3, 4, 5)),
            from((10, 20, 30, 40, 50)),
            from((60, 70, 80, 90, 100)),
            from((6, 7, 8, 9, 10)),
        ];

        let edits = diff_tokens(&before, &after);
        assert_eq!(
            edits[0],
            SemanticTokensEdit {
                start: 5,
                delete_count: 0,
                data: Some(vec![from((10, 20, 30, 40, 50)), from((60, 70, 80, 90, 100))])
            }
        );
    }

    #[test]
    fn test_diff_remove_from_end() {
        let before = [
            from((1, 2, 3, 4, 5)),
            from((6, 7, 8, 9, 10)),
            from((11, 12, 13, 14, 15)),
        ];
        let after = [from((1, 2, 3, 4, 5)), from((6, 7, 8, 9, 10))];

        let edits = diff_tokens(&before, &after);
        assert_eq!(
            edits[0],
            SemanticTokensEdit {
                start: 10,
                delete_count: 5,
                data: Some(vec![])
            }
        );
    }

    #[test]
    fn test_diff_remove_from_beginning() {
        let before = [
            from((11, 12, 13, 14, 15)),
            from((1, 2, 3, 4, 5)),
            from((6, 7, 8, 9, 10)),
        ];
        let after = [from((1, 2, 3, 4, 5)), from((6, 7, 8, 9, 10))];

        let edits = diff_tokens(&before, &after);
        assert_eq!(
            edits[0],
            SemanticTokensEdit {
                start: 0,
                delete_count: 5,
                data: Some(vec![])
            }
        );
    }

    #[test]
    fn test_diff_remove_from_middle() {
        let before = [
            from((1, 2, 3, 4, 5)),
            from((10, 20, 30, 40, 50)),
            from((60, 70, 80, 90, 100)),
            from((6, 7, 8, 9, 10)),
        ];
        let after = [from((1, 2, 3, 4, 5)), from((6, 7, 8, 9, 10))];

        let edits = diff_tokens(&before, &after);
        assert_eq!(
            edits[0],
            SemanticTokensEdit {
                start: 5,
                delete_count: 10,
                data: Some(vec![])
            }
        );
    }
}
