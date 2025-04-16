//! This module generates AST datatype used by rust-analyzer.
//!
//! Specifically, it generates the `SyntaxKind` enum and a number of newtype
//! wrappers around `SyntaxNode` which implement `syntax::AstNode`.

#![allow(unused)]

pub(crate) mod any_node_def;
mod ast_src;
mod lower_enum;

use crate::codegen::grammar::any_node_def::{
    extract_any_node_def, find_node_defs_with_trait, generate_any_node_def,
};
use crate::codegen::grammar::ast_src::{
    AstNodeSrc, AstSrc, Cardinality, Field, KINDS_SRC, KindsSrc, NON_METHOD_TRAITS, TRAITS,
    get_required_fields,
};
use crate::codegen::grammar::lower_enum::{generate_field_method_for_enum, lower_enum};
use crate::codegen::{add_preamble, ensure_file_contents, reformat};
use check_keyword::CheckKeyword;
use itertools::{Either, Itertools};
use proc_macro2::{Punct, Spacing};
use quote::{format_ident, quote};
use std::collections::{BTreeSet, HashSet};
use std::fmt::Write;
use std::ops::Index;
use stdx::panic_context;
use ungrammar::{Grammar, Rule};

pub fn generate() {
    let syntax_kinds = generate_syntax_kinds(KINDS_SRC);
    let syntax_kinds_file = crate::project_root().join("crates/parser/src/syntax_kind/generated.rs");
    ensure_file_contents(syntax_kinds_file.as_path(), &syntax_kinds);

    let grammar = include_str!("../../../crates/syntax/move.ungram")
        .parse::<Grammar>()
        .unwrap();
    let ast = lower(&grammar);

    let ast_tokens = generate_tokens(&ast);
    let ast_tokens_file = crate::project_root().join("crates/syntax/src/ast/generated/tokens.rs");
    ensure_file_contents(ast_tokens_file.as_path(), &ast_tokens);

    let ast_nodes = generate_nodes(KINDS_SRC, &ast);
    let ast_nodes_file = crate::project_root().join("crates/syntax/src/ast/generated/nodes.rs");
    ensure_file_contents(ast_nodes_file.as_path(), &ast_nodes);
}

fn generate_tokens(grammar: &AstSrc) -> String {
    let tokens = grammar.tokens.iter().map(|token| {
        let name = format_ident!("{}", token);
        let kind = format_ident!("{}", to_upper_snake_case(token));
        quote! {
            #[derive(Debug, Clone, PartialEq, Eq, Hash)]
            pub struct #name {
                pub(crate) syntax: SyntaxToken,
            }
            impl std::fmt::Display for #name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    std::fmt::Display::fmt(&self.syntax, f)
                }
            }
            impl AstToken for #name {
                fn can_cast(kind: SyntaxKind) -> bool { kind == #kind }
                fn cast(syntax: SyntaxToken) -> Option<Self> {
                    if Self::can_cast(syntax.kind()) { Some(Self { syntax }) } else { None }
                }
                fn syntax(&self) -> &SyntaxToken { &self.syntax }
            }
        }
    });

    add_preamble(
        "codegen",
        reformat(
            quote! {
                use crate::{SyntaxKind::{self, *}, SyntaxToken, ast::AstToken};
                #(#tokens)*
            }
            .to_string(),
        ),
    )
    .replace("#[derive", "\n#[derive")
}

fn generate_nodes(kinds: KindsSrc, grammar: &AstSrc) -> String {
    let (node_defs, node_boilerplate_impls): (Vec<_>, Vec<_>) = grammar
        .nodes
        .iter()
        .map(|node| {
            let name = format_ident!("{}", node.name);
            let kind = format_ident!("{}", to_upper_snake_case(&node.name));
            let traits = node
                .traits
                .iter()
                .filter(|trait_name| {
                    // Loops have two expressions so this might collide, therefore manual impl it
                    node.name != "ForExpr" && node.name != "WhileExpr"
                        || trait_name.as_str() != "HasLoopBody"
                })
                .map(|trait_name| {
                    let trait_name = format_ident!("{}", trait_name);
                    quote!(impl ast::#trait_name for #name {})
                });

            let methods = node
                .fields
                .iter()
                .map(|field| generate_field_method(name.to_string(), field));
            (
                quote! {
                    #[pretty_doc_comment_placeholder_workaround]
                    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
                    pub struct #name {
                        pub(crate) syntax: SyntaxNode,
                    }

                    #(#traits)*

                    impl #name {
                        #(#methods)*
                    }
                },
                quote! {
                    impl AstNode for #name {
                        #[inline]
                        fn kind() -> SyntaxKind
                        where
                            Self: Sized
                        {
                            #kind
                        }
                        #[inline]
                        fn can_cast(kind: SyntaxKind) -> bool {
                            kind == #kind
                        }
                        #[inline]
                        fn cast(syntax: SyntaxNode) -> Option<Self> {
                            if Self::can_cast(syntax.kind()) { Some(Self { syntax }) } else { None }
                        }
                        #[inline]
                        fn syntax(&self) -> &SyntaxNode { &self.syntax }
                    }
                },
            )
        })
        .unzip();

    let (enum_defs, enum_boilerplate_impls): (Vec<_>, Vec<_>) = grammar
        .enums
        .iter()
        .map(|enum_src| {
            let variants: Vec<_> = enum_src
                .variants
                .iter()
                .map(|var| format_ident!("{}", var))
                .sorted()
                .collect();
            let name = format_ident!("{}", enum_src.name);
            let kinds: Vec<_> = variants
                .iter()
                .map(|name| format_ident!("{}", to_upper_snake_case(&name.to_string())))
                .collect();
            let traits = enum_src.traits.iter().sorted().map(|trait_name| {
                let trait_name = format_ident!("{}", trait_name);
                quote!(impl ast::#trait_name for #name {})
            });

            let converters = variants.iter().map(|variant| {
                let variant_name = variant.to_owned();
                let mut lower_name = to_lower_snake_case(&variant_name.to_string());
                if lower_name.is_keyword() {
                    lower_name = format!("{}_", lower_name);
                }
                let lower_name = format_ident!("{}", lower_name);
                quote! {
                    pub fn #lower_name(self) -> Option<#variant_name> {
                        match (self) {
                            #name::#variant_name(item) => Some(item),
                            _ => None
                        }
                    }
                }
            });

            let common_fields = enum_src
                .common_fields
                .iter()
                .map(|common_field| generate_field_method_for_enum(enum_src, common_field))
                .collect::<Vec<_>>();

            let ast_node = quote! {
                impl #name {
                    #(#converters)*
                    #(#common_fields)*
                }
                impl AstNode for #name {
                    #[inline]
                    fn can_cast(kind: SyntaxKind) -> bool {
                        matches!(kind, #(#kinds)|*)
                    }
                    #[inline]
                    fn cast(syntax: SyntaxNode) -> Option<Self> {
                        let res = match syntax.kind() {
                            #(
                            #kinds => #name::#variants(#variants { syntax }),
                            )*
                            _ => return None,
                        };
                        Some(res)
                    }
                    #[inline]
                    fn syntax(&self) -> &SyntaxNode {
                        match self {
                            #(
                            #name::#variants(it) => &it.syntax,
                            )*
                        }
                    }
                }
            };

            (
                quote! {
                    #[pretty_doc_comment_placeholder_workaround]
                    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
                    pub enum #name {
                        #(#variants(#variants),)*
                    }

                    #(#traits)*
                },
                quote! {
                    #(
                        impl From<#variants> for #name {
                            #[inline]
                            fn from(node: #variants) -> #name {
                                #name::#variants(node)
                            }
                        }
                    )*
                    #ast_node
                },
            )
        })
        .unzip();

    let mut any_node_def_srcs = grammar
        .nodes
        .iter()
        .flat_map(|node| node.traits.iter().map(move |t| (t, node)))
        .into_group_map()
        .into_iter()
        .sorted_by_key(|(name, _)| *name)
        .map(|(trait_name, nodes)| extract_any_node_def(trait_name, nodes))
        .collect::<Vec<_>>();

    for any_node_def_src in any_node_def_srcs.clone() {
        let current_trait_name = any_node_def_src.trait_name;
        let other_defs_with_trait =
            find_node_defs_with_trait(&current_trait_name, any_node_def_srcs.clone());
        if let Some(any_def) = any_node_def_srcs
            .iter_mut()
            .find(|it| it.trait_name == current_trait_name)
        {
            for other_def in other_defs_with_trait {
                any_def.from_impls.push(format!("Any{}", other_def.trait_name));
            }
        }
    }

    let (any_node_defs, any_node_boilerplate_impls): (Vec<_>, Vec<_>) = any_node_def_srcs
        .into_iter()
        .map(|it| generate_any_node_def(it))
        .unzip();

    let enum_names = grammar.enums.iter().map(|it| &it.name);
    let node_names = grammar.nodes.iter().map(|it| &it.name);

    let display_impls = enum_names
        .chain(node_names.clone())
        .map(|it| format_ident!("{}", it))
        .map(|name| {
            quote! {
                impl std::fmt::Display for #name {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        std::fmt::Display::fmt(self.syntax(), f)
                    }
                }
            }
        });

    let defined_nodes: HashSet<_> = node_names.collect();

    for node in kinds
        .nodes
        .iter()
        .map(|kind| to_pascal_case(kind))
        .filter(|name| !defined_nodes.iter().any(|&it| it == name))
    {
        drop(node)
        // FIXME: restore this
        // eprintln!("Warning: node {} not defined in ast source", node);
    }

    let ast = quote! {
        #![allow(non_snake_case)]
        use crate::{
            SyntaxNode, SyntaxToken, SyntaxKind::{self, *},
            ast::{self, AstNode, AstChildren, support},
            T,
        };

        #(#node_defs)*
        #(#enum_defs)*
        #(#any_node_defs)*

        #(#node_boilerplate_impls)*
        #(#enum_boilerplate_impls)*
        #(#any_node_boilerplate_impls)*
        #(#display_impls)*
    };

    let ast = ast.to_string().replace("T ! [", "T![");

    let mut res = String::with_capacity(ast.len() * 2);

    let mut docs = grammar
        .nodes
        .iter()
        .map(|it| &it.doc)
        .chain(grammar.enums.iter().map(|it| &it.doc));

    for chunk in ast.split("# [pretty_doc_comment_placeholder_workaround] ") {
        res.push_str(chunk);
        if let Some(doc) = docs.next() {
            write_doc_comment(doc, &mut res);
        }
    }

    let res = add_preamble("codegen", reformat(res));
    res.replace("#[derive", "\n#[derive")
}

fn generate_field_method(node_name: String, field: &Field) -> proc_macro2::TokenStream {
    let method_name = format_ident!("{}", field.method_name());
    let expect_line =
        proc_macro2::Literal::string(&format!("{}.{} required by the parser", node_name, method_name));
    let ty = field.ty();
    match field {
        Field::Node { cardinality, .. } => match cardinality {
            Cardinality::Many => {
                quote! {
                    #[inline]
                    pub fn #method_name(&self) -> AstChildren<#ty> {
                        support::children(&self.syntax)
                    }
                }
            }
            Cardinality::Required => {
                quote! {
                    #[inline]
                    pub fn #method_name(&self) -> #ty {
                        support::child(&self.syntax).expect(#expect_line)
                    }
                }
            }
            Cardinality::Optional => {
                quote! {
                    #[inline]
                    pub fn #method_name(&self) -> Option<#ty> {
                        support::child(&self.syntax)
                    }
                }
            }
        },
        Field::Token { name, cardinality } => {
            let token: proc_macro2::TokenStream = name.parse().unwrap();
            let token_kind = quote! { T![#token] };
            match cardinality {
                Cardinality::Required => {
                    quote! {
                        #[inline]
                        pub fn #method_name(&self) -> #ty {
                            support::token(&self.syntax, #token_kind).expect(#expect_line)
                        }
                    }
                }
                Cardinality::Optional => {
                    quote! {
                        #[inline]
                        pub fn #method_name(&self) -> Option<#ty> {
                            support::token(&self.syntax, #token_kind)
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}

fn write_doc_comment(contents: &[String], dest: &mut String) {
    for line in contents {
        dest.write_fmt(format_args!("///{}", line)).unwrap();
        // writeln!(dest, "///{}", line).unwrap();
    }
}

fn generate_syntax_kinds(grammar: KindsSrc<'_>) -> String {
    let (single_byte_tokens_values, single_byte_tokens): (Vec<_>, Vec<_>) = grammar
        .punct
        .iter()
        .filter(|(token, _name)| token.len() == 1)
        .map(|(token, name)| (token.chars().next().unwrap(), format_ident!("{}", name)))
        .unzip();

    let punctuation_values = grammar.punct.iter().map(|(token, _name)| {
        if "{}[]()_".contains(token) {
            let c = token.chars().next().unwrap();
            quote! { #c }
        } else {
            let cs = token.chars().map(|c| Punct::new(c, Spacing::Joint));
            quote! { #(#cs)* }
        }
    });
    let punctuation = grammar
        .punct
        .iter()
        .map(|(_token, name)| format_ident!("{}", name))
        .collect::<Vec<_>>();

    let full_keywords_values = &grammar.keywords;
    let full_keywords = full_keywords_values
        .iter()
        .map(|kw| format_ident!("{}_KW", to_upper_snake_case(kw)));

    let all_keywords_values = grammar
        .keywords
        .iter()
        .chain(grammar.contextual_keywords.iter())
        .collect::<Vec<_>>();
    let all_keywords_idents = all_keywords_values.iter().map(|kw| format_ident!("{}", kw));
    let all_keywords = all_keywords_values
        .iter()
        .map(|name| format_ident!("{}_KW", to_upper_snake_case(name)))
        .collect::<Vec<_>>();

    let literals = grammar
        .literals
        .iter()
        .map(|name| format_ident!("{}", name))
        .collect::<Vec<_>>();

    let tokens = grammar
        .tokens
        .iter()
        .map(|name| format_ident!("{}", name))
        .collect::<Vec<_>>();

    let nodes = grammar
        .nodes
        .iter()
        .map(|name| format_ident!("{}", name))
        .collect::<Vec<_>>();

    let ast = quote! {
        #![allow(bad_style, missing_docs, unreachable_pub)]
        /// The kind of syntax node, e.g. `IDENT`, `USE_KW`, or `STRUCT`.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        #[repr(u16)]
        pub enum SyntaxKind {
            // Technical SyntaxKinds: they appear temporally during parsing,
            // but never end up in the final tree
            #[doc(hidden)]
            TOMBSTONE,
            #[doc(hidden)]
            EOF,
            #(#punctuation,)*
            #(#all_keywords,)*
            #(#literals,)*
            #(#tokens,)*
            #(#nodes,)*

            // Technical kind so that we can cast from u16 safely
            #[doc(hidden)]
            __LAST,
        }
        use self::SyntaxKind::*;

        impl SyntaxKind {
            pub fn is_keyword(self) -> bool {
                match self {
                    #(#all_keywords)|* => true,
                    _ => false,
                }
            }

            pub fn is_punct(self) -> bool {
                match self {
                    #(#punctuation)|* => true,
                    _ => false,
                }
            }

            pub fn is_literal(self) -> bool {
                match self {
                    #(#literals)|* => true,
                    _ => false,
                }
            }

            pub fn from_keyword(ident: &str) -> Option<SyntaxKind> {
                let kw = match ident {
                    #(#full_keywords_values => #full_keywords,)*
                    _ => return None,
                };
                Some(kw)
            }

            pub fn from_char(c: char) -> Option<SyntaxKind> {
                let tok = match c {
                    #(#single_byte_tokens_values => #single_byte_tokens,)*
                    _ => return None,
                };
                Some(tok)
            }
        }

        #[macro_export]
        macro_rules! T {
            #([#punctuation_values] => { $crate::SyntaxKind::#punctuation };)*
            #([#all_keywords_idents] => { $crate::SyntaxKind::#all_keywords };)*
            [quote_ident] => { $crate::SyntaxKind::QUOTE_IDENT };
            [ident] => { $crate::SyntaxKind::IDENT };
            [int_number] => { $crate::SyntaxKind::INT_NUMBER };
            [hex_string] => { $crate::SyntaxKind::HEX_STRING };
            [byte_string] => { $crate::SyntaxKind::BYTE_STRING };
        }
        #[allow(unused_imports)]
        pub use T;
    };

    add_preamble("codegen", reformat(ast.to_string()))
}

fn lower(grammar: &Grammar) -> AstSrc {
    let mut res = AstSrc {
        tokens: "Whitespace Comment IntNumber Ident ByteString HexString"
            .split_ascii_whitespace()
            .map(|it| it.to_owned())
            .collect::<Vec<_>>(),
        ..Default::default()
    };

    let nodes = grammar.iter().collect::<Vec<_>>();

    for &node in &nodes {
        let node_name = grammar[node].name.clone();
        let rule = &grammar[node].rule;
        let _g = panic_context::enter(node_name.clone());
        match lower_enum(grammar, node_name.as_str(), rule) {
            Some(enum_src) => {
                res.enums.push(enum_src);
            }
            None => {
                let mut fields = Vec::new();
                let required_fields = get_required_fields(node_name.as_str());
                lower_rule(&mut fields, grammar, None, rule, required_fields);
                res.nodes.push(AstNodeSrc {
                    doc: Vec::new(),
                    name: node_name,
                    traits: Vec::new(),
                    fields,
                });
            }
        }
    }

    deduplicate_fields(&mut res);
    // extract_enums(&mut res);
    extract_struct_traits(&mut res);
    extract_enum_traits(&mut res);
    res.nodes.sort_by_key(|it| it.name.clone());
    res.enums.sort_by_key(|it| it.name.clone());
    res.tokens.sort();
    res.nodes.iter_mut().for_each(|it| {
        it.traits.sort();
        it.fields.sort_by_key(|it| match it {
            Field::Token { name, .. } => (true, name.clone()),
            Field::Node { name, .. } => (false, name.clone()),
        });
    });
    res.enums.iter_mut().for_each(|it| {
        it.traits.sort();
        it.variants.sort();
    });
    res
}

fn lower_rule(
    acc: &mut Vec<Field>,
    grammar: &Grammar,
    label: Option<&String>,
    rule: &Rule,
    required_fields: &[&str],
) {
    if lower_separated_list(acc, grammar, label, rule) {
        return;
    }

    let get_rule_cardinality = |rule_name: &str| {
        if (required_fields.contains(&rule_name)) {
            Cardinality::Required
        } else {
            Cardinality::Optional
        }
    };

    match rule {
        Rule::Node(node) => {
            let ty = grammar[*node].name.clone();
            let name = label.cloned().unwrap_or_else(|| to_lower_snake_case(&ty));
            let cardinality = get_rule_cardinality(&name);
            let field = Field::Node { name, ty, cardinality };
            acc.push(field);
        }
        Rule::Token(token) => {
            assert!(label.is_none());
            let mut name = clean_token_name(&grammar[*token].name);
            if "[]{}()_".contains(&name) {
                name = format!("'{name}'");
            }
            let cardinality = get_rule_cardinality(&name);
            let field = Field::Token { name, cardinality };
            acc.push(field);
        }
        Rule::Rep(inner) => {
            if let Rule::Node(node) = &**inner {
                let ty = grammar[*node].name.clone();
                let name = label
                    .cloned()
                    .unwrap_or_else(|| pluralize(&to_lower_snake_case(&ty)));
                let field = Field::Node {
                    name,
                    ty,
                    cardinality: Cardinality::Many,
                };
                acc.push(field);
                return;
            }
            panic!("unhandled rule: {rule:?}")
        }
        Rule::Labeled { label: l, rule } => {
            assert!(label.is_none());
            let manually_implemented = matches!(
                l.as_str(),
                "lhs"
                    | "op"
                    | "rhs"
                    | "then_branch"
                    | "else_branch"
                    | "loop_body_expr"
                    | "start_expr"
                    | "end_expr"
                    | "base_expr"
                    | "arg_expr" // | "value"
                                 // | "trait"
                                 // | "self_ty"
                                 // | "iterable"
                                 // | "condition"
                                 // | "args"
                                 // | "body"
            );
            if manually_implemented {
                return;
            }
            lower_rule(acc, grammar, Some(l), rule, required_fields);
        }
        Rule::Seq(rules) | Rule::Alt(rules) => {
            for rule in rules {
                lower_rule(acc, grammar, label, rule, required_fields)
            }
        }
        Rule::Opt(rule) => lower_rule(acc, grammar, label, rule, required_fields),
    }
}

// (T (',' T)* ','?)
fn lower_separated_list(
    acc: &mut Vec<Field>,
    grammar: &Grammar,
    label: Option<&String>,
    rule: &Rule,
) -> bool {
    let rule = match rule {
        Rule::Seq(it) => it,
        _ => return false,
    };

    let (nt, repeat, trailing_sep) = match rule.as_slice() {
        [Rule::Node(node), Rule::Rep(repeat), Rule::Opt(trailing_sep)] => {
            (Either::Left(node), repeat, Some(trailing_sep))
        }
        [Rule::Node(node), Rule::Rep(repeat)] => (Either::Left(node), repeat, None),
        [Rule::Token(token), Rule::Rep(repeat), Rule::Opt(trailing_sep)] => {
            (Either::Right(token), repeat, Some(trailing_sep))
        }
        [Rule::Token(token), Rule::Rep(repeat)] => (Either::Right(token), repeat, None),
        _ => return false,
    };
    let repeat = match &**repeat {
        Rule::Seq(it) => it,
        _ => return false,
    };
    if !matches!(
        repeat.as_slice(),
        [comma, nt_]
            if trailing_sep.is_none_or(|it| comma == &**it) && match (nt, nt_) {
                (Either::Left(node), Rule::Node(nt_)) => node == nt_,
                (Either::Right(token), Rule::Token(nt_)) => token == nt_,
                _ => false,
            }
    ) {
        return false;
    }
    match nt {
        Either::Right(token) => {
            let name = clean_token_name(&grammar[*token].name);
            let field = Field::Token {
                name,
                cardinality: Cardinality::Optional,
            };
            acc.push(field);
        }
        Either::Left(node) => {
            let ty = grammar[*node].name.clone();
            let name = label
                .cloned()
                .unwrap_or_else(|| pluralize(&to_lower_snake_case(&ty)));
            let field = Field::Node {
                name,
                ty,
                cardinality: Cardinality::Many,
            };
            acc.push(field);
        }
    }
    true
}

fn deduplicate_fields(ast: &mut AstSrc) {
    for node in &mut ast.nodes {
        let mut i = 0;
        'outer: while i < node.fields.len() {
            for j in 0..i {
                let f1 = &node.fields[i];
                let f2 = &node.fields[j];
                if f1 == f2 {
                    node.fields.remove(i);
                    continue 'outer;
                }
            }
            i += 1;
        }
    }
}

fn extract_enums(ast: &mut AstSrc) {
    for node in &mut ast.nodes {
        for enum_ in &ast.enums {
            let mut to_remove = Vec::new();
            for (i, field) in node.fields.iter().enumerate() {
                let ty = field.ty().to_string();
                if enum_.variants.iter().any(|it| it == &ty) {
                    to_remove.push(i);
                }
            }
            if to_remove.len() == enum_.variants.len() {
                node.remove_field(to_remove);
                let ty = enum_.name.clone();
                let name = to_lower_snake_case(&ty);
                node.fields.push(Field::Node {
                    name,
                    ty,
                    cardinality: Cardinality::Optional,
                });
            }
        }
    }
}

fn extract_struct_traits(ast: &mut AstSrc) {
    for node in &mut ast.nodes {
        for (name, methods) in TRAITS {
            extract_struct_trait(node, name, methods);
        }
    }

    for node in &mut ast.nodes {
        for (trait_name, node_names) in NON_METHOD_TRAITS {
            if node_names.contains(&&*node.name) {
                node.traits.push((*trait_name).into());
            }
        }
    }
}

fn extract_struct_trait(node: &mut AstNodeSrc, trait_name: &str, methods: &[&str]) {
    let mut to_remove = Vec::new();
    for (i, field) in node.fields.iter().enumerate() {
        let method_name = field.method_name();
        if methods.iter().any(|&it| it == method_name) {
            to_remove.push(i);
        }
    }
    if to_remove.len() == methods.len() {
        node.traits.push(trait_name.to_owned());
        node.remove_field(to_remove);
    }
}

fn extract_enum_traits(ast: &mut AstSrc) {
    for enum_ in &mut ast.enums {
        let nodes = &ast.nodes;

        let nodes = enum_
            .variants
            .iter()
            .map(|variant| nodes.iter().find(|it| &it.name == variant).unwrap())
            .collect::<Vec<_>>();

        let enum_traits = find_common_traits(nodes);
        if !enum_traits.is_empty() {
            enum_.traits = enum_traits;
        }
    }
}

fn find_common_traits(nodes: Vec<&AstNodeSrc>) -> Vec<String> {
    let mut variant_traits = nodes
        .into_iter()
        .map(|node| node.traits.iter().cloned().collect::<BTreeSet<_>>());
    // collect traits present on all the variants
    let mut enum_traits = match variant_traits.next() {
        Some(it) => it,
        None => return vec![],
    };
    for traits in variant_traits {
        enum_traits = enum_traits.intersection(&traits).cloned().collect();
    }
    enum_traits.into_iter().collect()
}

fn to_upper_snake_case(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    let mut prev = false;
    for c in s.chars() {
        if c.is_ascii_uppercase() && prev {
            buf.push('_')
        }
        prev = true;

        buf.push(c.to_ascii_uppercase());
    }
    buf
}

fn to_lower_snake_case(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    let mut prev = false;
    for c in s.chars() {
        if c.is_ascii_uppercase() && prev {
            buf.push('_')
        }
        prev = true;

        buf.push(c.to_ascii_lowercase());
    }
    buf
}

fn to_pascal_case(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    let mut prev_is_underscore = true;
    for c in s.chars() {
        if c == '_' {
            prev_is_underscore = true;
        } else if prev_is_underscore {
            buf.push(c.to_ascii_uppercase());
            prev_is_underscore = false;
        } else {
            buf.push(c.to_ascii_lowercase());
        }
    }
    buf
}

fn pluralize(s: &str) -> String {
    if s.ends_with("y") {
        return format!("{}ies", s.strip_suffix("y").unwrap());
    }
    format!("{s}s")
}

impl Field {
    fn token_kind(&self) -> Option<proc_macro2::TokenStream> {
        match self {
            Field::Token { name, .. } => {
                let token: proc_macro2::TokenStream = name.parse().unwrap();
                Some(quote! { T![#token] })
            }
            _ => None,
        }
    }
    fn method_name(&self) -> String {
        match self {
            Field::Token { name, .. } => {
                let name = match name.as_str() {
                    ";" => "semicolon",
                    "->" => "thin_arrow",
                    "'{'" => "l_curly",
                    "'}'" => "r_curly",
                    "'('" => "l_paren",
                    "')'" => "r_paren",
                    "'['" => "l_brack",
                    "']'" => "r_brack",
                    "'_'" => "underscore",
                    "<" => "l_angle",
                    ">" => "r_angle",
                    "=" => "eq",
                    "!" => "excl",
                    "*" => "star",
                    "&" => "amp",
                    "." => "dot",
                    ".." => "dotdot",
                    "..." => "dotdotdot",
                    "..=" => "dotdoteq",
                    "=>" => "fat_arrow",
                    "@" => "at",
                    ":" => "colon",
                    "::" => "coloncolon",
                    "#" => "pound",
                    "?" => "question_mark",
                    "," => "comma",
                    "|" => "pipe",
                    "~" => "tilde",
                    _ => name,
                };
                format!("{}_token", name)
            }
            Field::Node { name, .. } => {
                if name == "type" {
                    "type_".to_string()
                } else {
                    format!("{}", name)
                }
            }
        }
    }
    fn ty(&self) -> proc_macro2::Ident {
        match self {
            Field::Token { .. } => format_ident!("SyntaxToken"),
            Field::Node { ty, .. } => format_ident!("{}", ty),
        }
    }
}

fn clean_token_name(name: &str) -> String {
    let cleaned = name.trim_start_matches(['@', '#', '?']);
    if cleaned.is_empty() {
        name.to_owned()
    } else {
        cleaned.to_owned()
    }
}

impl AstNodeSrc {
    fn remove_field(&mut self, to_remove: Vec<usize>) {
        to_remove.into_iter().rev().for_each(|idx| {
            self.fields.remove(idx);
        });
    }
}
