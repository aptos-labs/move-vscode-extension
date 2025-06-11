use ide_db::SymbolKind;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Highlight {
    pub tag: HlTag,
    // pub mods: HlMods,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HlTag {
    Symbol(SymbolKind),

    // AttributeBracket,
    BoolLiteral,
    BuiltinType,
    // ByteLiteral,
    // CharLiteral,
    Comment,
    // EscapeSequence,
    // InvalidEscapeSequence,
    Keyword,
    NumericLiteral,
    // Operator(HlOperator),
    // Punctuation(HlPunct),
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
                // SymbolKind::BuiltinAttr => "builtin_attr",
                SymbolKind::Const => "constant",
                // SymbolKind::ConstParam => "const_param",
                // SymbolKind::Derive => "derive",
                // SymbolKind::DeriveHelper => "derive_helper",
                SymbolKind::Enum => "enum",
                SymbolKind::Field => "field",
                SymbolKind::Function => "function",
                // SymbolKind::Impl => "self_type",
                // SymbolKind::InlineAsmRegOrRegClass => "reg",
                SymbolKind::Label => "label",
                // SymbolKind::LifetimeParam => "lifetime",
                SymbolKind::Local => "variable",
                // SymbolKind::Macro => "macro",
                SymbolKind::Method => "method",
                // SymbolKind::ProcMacro => "proc_macro",
                SymbolKind::Module => "module",
                // SymbolKind::SelfParam => "self_keyword",
                // SymbolKind::SelfType => "self_type_keyword",
                // SymbolKind::Static => "static",
                SymbolKind::Struct => "struct",
                // SymbolKind::ToolModule => "tool_module",
                // SymbolKind::Trait => "trait",
                // SymbolKind::TraitAlias => "trait_alias",
                // SymbolKind::TypeAlias => "type_alias",
                SymbolKind::TypeParam => "type_param",
                // SymbolKind::Union => "union",
                SymbolKind::ValueParam => "value_param",
                SymbolKind::EnumVariant => "enum_variant",
                SymbolKind::GlobalVariableDecl => "global",
                SymbolKind::Vector => "vector",
            },
            // HlTag::AttributeBracket => "attribute_bracket",
            HlTag::BoolLiteral => "bool_literal",
            HlTag::BuiltinType => "builtin_type",
            // HlTag::ByteLiteral => "byte_literal",
            // HlTag::CharLiteral => "char_literal",
            HlTag::Comment => "comment",
            // HlTag::EscapeSequence => "escape_sequence",
            // HlTag::InvalidEscapeSequence => "invalid_escape_sequence",
            // HlTag::FormatSpecifier => "format_specifier",
            HlTag::Keyword => "keyword",
            // HlTag::Punctuation(punct) => match punct {
            //     HlPunct::Bracket => "bracket",
            //     HlPunct::Brace => "brace",
            //     HlPunct::Parenthesis => "parenthesis",
            //     HlPunct::Angle => "angle",
            //     HlPunct::Comma => "comma",
            //     HlPunct::Dot => "dot",
            //     HlPunct::Colon => "colon",
            //     HlPunct::Semi => "semicolon",
            //     HlPunct::MacroBang => "macro_bang",
            //     HlPunct::Other => "punctuation",
            // },
            HlTag::NumericLiteral => "numeric_literal",
            // HlTag::Operator(op) => match op {
            //     HlOperator::Bitwise => "bitwise",
            //     HlOperator::Arithmetic => "arithmetic",
            //     HlOperator::Logical => "logical",
            //     HlOperator::Comparison => "comparison",
            //     HlOperator::Other => "operator",
            // },
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
        self.tag.fmt(f)?;
        // for modifier in self.mods.iter() {
        //     f.write_char('.')?;
        //     modifier.fmt(f)?;
        // }
        Ok(())
    }
}

impl From<HlTag> for Highlight {
    fn from(tag: HlTag) -> Highlight {
        Highlight::new(tag)
    }
}

impl Highlight {
    pub(crate) fn new(tag: HlTag) -> Highlight {
        Highlight {
            tag, /*mods: HlMods::default()*/
        }
    }
    pub fn is_empty(&self) -> bool {
        self.tag == HlTag::None /*&& self.mods.is_empty()*/
    }
}
