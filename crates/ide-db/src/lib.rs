#![allow(dead_code)]

pub mod apply_change;
pub mod assist_config;
pub mod assists;
pub mod defs;
pub mod helpers;
pub mod label;
pub mod load;
pub mod root_db;
pub mod source_change;
mod syntax_helpers;
pub mod text_edit;

use syntax::{SyntaxKind, SyntaxKind::*};

pub use root_db::RootDatabase;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolKind {
    Attribute,
    Module,
    Const,
    Function,
    Method,
    Struct,
    Field,
    Enum,
    EnumVariant,
    TypeParam,
    ValueParam,
    Label,
    Local,
    // SpecFunction,
    // SpecInlineFunction,
    // Schema,
    // ModuleSpec,
    // ItemSpec,
    GlobalVariableDecl,
}

pub fn ast_kind_to_symbol_kind(kind: SyntaxKind) -> Option<SymbolKind> {
    match kind {
        MODULE => Some(SymbolKind::Module),

        FUN | SPEC_FUN | SPEC_INLINE_FUN => Some(SymbolKind::Function),

        CONST => Some(SymbolKind::Const),
        STRUCT => Some(SymbolKind::Struct),
        ENUM => Some(SymbolKind::Enum),

        TYPE_PARAM => Some(SymbolKind::TypeParam),
        IDENT_PAT => Some(SymbolKind::Local),
        VARIANT => Some(SymbolKind::EnumVariant),

        NAMED_FIELD => Some(SymbolKind::Field),

        SCHEMA => Some(SymbolKind::Struct),
        SCHEMA_FIELD => Some(SymbolKind::Field),
        GLOBAL_VARIABLE_DECL => Some(SymbolKind::GlobalVariableDecl),

        USE_ALIAS => Some(SymbolKind::Local),

        _ => {
            tracing::error!("unhandled ast kind {:?}", kind);
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AllowSnippets {
    _private: (),
}

impl AllowSnippets {
    pub const fn new(allow_snippets: bool) -> Option<AllowSnippets> {
        if allow_snippets {
            Some(AllowSnippets { _private: () })
        } else {
            None
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    WeakWarning,
    Allow,
}

impl Severity {
    pub fn from_test_ident(ident: &str) -> Severity {
        let expected_severity = match ident {
            "err:" => Severity::Error,
            "warn:" => Severity::Warning,
            "weak:" => Severity::WeakWarning,
            "allow:" => Severity::Allow,
            _ => unreachable!("unknown severity {:?}", ident),
        };
        expected_severity
    }

    pub fn to_test_ident(&self) -> &str {
        match self {
            Severity::Error => "err:",
            Severity::Warning => "warn",
            Severity::WeakWarning => "weak:",
            Severity::Allow => "allow:",
        }
    }
}
