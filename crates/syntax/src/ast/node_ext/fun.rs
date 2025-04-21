use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::ast::NamedElement;
use crate::SyntaxKind::*;
use parser::SyntaxKind;
use std::collections::HashSet;

impl ast::Fun {
    pub fn modifiers(&self) -> HashSet<SyntaxKind> {
        let mut modifiers = HashSet::new();
        let vis_modifier = self.visibility_modifier();
        if let Some(vis_modifier) = vis_modifier {
            if vis_modifier.is_public() {
                modifiers.insert(PUBLIC_KW);
            }
            if vis_modifier.is_friend() {
                modifiers.insert(FRIEND_KW);
            }
            if vis_modifier.is_package() {
                modifiers.insert(PACKAGE_KW);
            }
        }
        if self.is_inline() {
            modifiers.insert(INLINE_KW);
        }
        if self.is_entry() {
            modifiers.insert(ENTRY_KW);
        }
        if self.is_native() {
            modifiers.insert(NATIVE_KW);
        }
        modifiers
    }

    pub fn modifiers_as_strings(&self) -> HashSet<String> {
        self.modifiers()
            .into_iter()
            .map(|it| {
                match it {
                    PUBLIC_KW => "public",
                    INLINE_KW => "inline",
                    ENTRY_KW => "entry",
                    FRIEND_KW => "friend",
                    PACKAGE_KW => "package",
                    NATIVE_KW => "native",
                    _ => unreachable!(),
                }
                .to_string()
            })
            .collect()
    }

    pub fn params(&self) -> Vec<ast::Param> {
        self.param_list()
            .map(|list| list.params().collect())
            .unwrap_or_default()
    }

    pub fn params_as_bindings(&self) -> Vec<ast::IdentPat> {
        self.params()
            .into_iter()
            .filter_map(|param| param.ident_pat())
            .collect()
    }

    pub fn return_type(&self) -> Option<ast::Type> {
        self.ret_type()?.type_()
    }

    pub fn is_native(&self) -> bool {
        self.native_token().is_some()
    }
    pub fn is_entry(&self) -> bool {
        self.entry_token().is_some()
    }
    pub fn is_inline(&self) -> bool {
        self.inline_token().is_some()
    }

    pub fn self_param(&self) -> Option<ast::Param> {
        let param = self.params().first()?.to_owned();
        if param.ident_name() != "self" {
            return None;
        }
        Some(param)
    }
}
