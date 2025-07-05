// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

#![allow(dead_code)]

pub mod active_parameter;
pub mod apply_change;
pub mod assist_config;
pub mod assist_context;
pub mod assists;
pub mod defs;
pub mod helpers;
pub mod label;
pub mod load;
pub mod rename;
pub mod root_db;
pub mod search;
pub mod source_change;
pub mod symbol_index;
mod syntax_helpers;
pub mod text_edit;

use syntax::{AstNode, SyntaxKind, ast};

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
    Vector,
    Assert,
    // SpecFunction,
    // SpecInlineFunction,
    // Schema,
    // ModuleSpec,
    // ItemSpec,
    GlobalVariableDecl,
}

pub fn ast_kind_to_symbol_kind(named_item: &ast::NamedElement) -> SymbolKind {
    let item_parent = named_item.syntax().parent().map(|it| it.kind());
    match named_item {
        ast::NamedElement::Module(_) => SymbolKind::Module,

        ast::NamedElement::Fun(_)
        | ast::NamedElement::SpecFun(_)
        | ast::NamedElement::SpecInlineFun(_) => SymbolKind::Function,

        ast::NamedElement::Const(_) => SymbolKind::Const,

        ast::NamedElement::Struct(_) => SymbolKind::Struct,
        ast::NamedElement::Enum(_) => SymbolKind::Enum,

        ast::NamedElement::TypeParam(_) => SymbolKind::TypeParam,
        ast::NamedElement::Variant(_) => SymbolKind::EnumVariant,

        ast::NamedElement::NamedField(_) => SymbolKind::Field,

        ast::NamedElement::Schema(_) => SymbolKind::Struct,
        ast::NamedElement::GlobalVariableDecl(_) => SymbolKind::GlobalVariableDecl,
        ast::NamedElement::IdentPat(_) if matches!(item_parent, Some(SyntaxKind::SCHEMA_FIELD)) => {
            SymbolKind::Field
        }

        ast::NamedElement::IdentPat(_) => SymbolKind::Local,
        ast::NamedElement::UseAlias(_) => SymbolKind::Local,
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
            Severity::Warning => "warn:",
            Severity::WeakWarning => "weak:",
            Severity::Allow => "allow:",
        }
    }
}
