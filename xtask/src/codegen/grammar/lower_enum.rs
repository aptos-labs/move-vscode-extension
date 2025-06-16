use crate::codegen::grammar::ast_src::{AstNodeSrc, AstSrc, Cardinality, Field, get_required_fields};
use crate::codegen::grammar::{find_common_traits, lower_rule};
use quote::{format_ident, quote};
use std::collections::HashSet;
use std::env::var;
use std::ops::Deref;
use stdx::panic_context;
use ungrammar::{Grammar, Node, Rule};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct AstEnumSrc {
    pub(crate) doc: Vec<String>,
    pub(crate) name: String,
    pub(crate) traits: Vec<String>,
    pub(crate) variants: Vec<String>,
    pub(crate) transitive_variants: Vec<(String, String)>,
    pub(crate) common_enums: Vec<String>,
    pub(crate) common_fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub(crate) enum NodeKind {
    Node(Node),
    Enum { root: Node, variants: Vec<Node> },
}

pub(super) fn node_kind(grammar: &Grammar, node: &Node) -> NodeKind {
    let root_rule = &grammar[*node].rule;
    let alternatives = match root_rule {
        Rule::Alt(it) => it,
        _ => return NodeKind::Node(node.clone()),
    };

    let mut alt_rules = vec![];
    for alternative in alternatives {
        match alternative {
            Rule::Node(it) => {
                alt_rules.push(it.clone());
            }
            Rule::Token(it) if grammar[*it].name == ";" => (),
            _ => return NodeKind::Node(node.clone()),
        }
    }

    NodeKind::Enum {
        root: node.clone(),
        variants: alt_rules,
    }
}

pub(super) fn lower_enum(
    grammar: &Grammar,
    root: Node,
    variant_nodes: Vec<Node>,
    ast: &AstSrc,
) -> AstEnumSrc {
    let enum_name = grammar[root.clone()].name.clone();
    let _g = panic_context::enter(enum_name.clone());

    let mut variants = Vec::new();
    let mut variant_field_sets = vec![];

    for variant_node in variant_nodes {
        let (variant_name, variant_fields) = lower_variant_node(grammar, &variant_node);
        variants.push(variant_name);
        variant_field_sets.push(variant_fields);
    }
    let enum_common_fields = collect_common_fields(variant_field_sets);

    let enum_src = AstEnumSrc {
        doc: vec![],
        name: enum_name.to_string(),
        traits: vec![],
        variants,
        transitive_variants: vec![],
        common_fields: enum_common_fields,
        common_enums: vec![],
    };
    enum_src
}

fn lower_variant_node(grammar: &Grammar, node: &Node) -> (String, Vec<Field>) {
    let node_data = &grammar[*node];
    let variant_name = node_data.name.clone();
    let required_fields = get_required_fields(&variant_name);
    let mut variant_fields = vec![];
    match &node_data.rule {
        Rule::Node(_) => {
            lower_rule(
                &mut variant_fields,
                grammar,
                None,
                &node_data.rule,
                required_fields,
            );
        }
        Rule::Seq(rules) => {
            for child_rule in rules {
                match child_rule {
                    Rule::Opt(rule) => {
                        if let Rule::Node(_) = rule.deref() {
                            lower_rule(&mut variant_fields, grammar, None, child_rule, required_fields);
                        }
                    }
                    Rule::Node(node) => {
                        lower_rule(&mut variant_fields, grammar, None, child_rule, required_fields);
                    }
                    Rule::Labeled { label, rule } => match rule.deref() {
                        Rule::Node(_) => {
                            lower_rule(&mut variant_fields, grammar, Some(label), rule, required_fields);
                        }
                        Rule::Opt(rule) => {
                            lower_rule(&mut variant_fields, grammar, Some(label), rule, required_fields);
                        }
                        _ => (),
                    },
                    _ => (),
                }
            }
        }
        _ => (),
    }
    (variant_name, variant_fields)
}

fn collect_common_fields(variant_field_sets: Vec<Vec<Field>>) -> Vec<Field> {
    let mut common_fields: Option<Vec<Field>> = None;
    for variant_fields in variant_field_sets {
        match common_fields {
            None => {
                common_fields = Some(variant_fields);
            }
            Some(fields) => {
                let mut present_common_fields = vec![];
                for common_field in fields {
                    if variant_fields.contains(&common_field) {
                        present_common_fields.push(common_field);
                    }
                }
                common_fields = Some(present_common_fields);
            }
        }
    }
    common_fields.unwrap_or_default()
}

pub(super) fn extract_common_enum_traits(ast: &mut AstSrc) {
    let ast_clone = ast.clone();
    for enum_ in &mut ast.enums {
        let variants_src = enum_
            .variants
            .iter()
            .filter_map(|variant| find_node_src(&ast_clone, variant))
            .collect::<Vec<_>>();

        let enum_traits = find_common_traits(variants_src);
        if !enum_traits.is_empty() {
            enum_.traits = enum_traits;
        }
    }
}

pub(crate) fn get_enums_for_node(node_name: &String, enums: &Vec<AstEnumSrc>) -> HashSet<String> {
    let mut res = HashSet::new();
    for e in enums.iter() {
        if e.variants.contains(node_name) {
            res.insert(e.name.clone());
        }
    }
    res
}

fn find_node_src<'src>(ast: &'src AstSrc, node_name: &str) -> Option<&'src AstNodeSrc> {
    ast.nodes.iter().find(|it| it.name == node_name)
}

pub(super) fn generate_field_method_for_enum(
    enum_src: &AstEnumSrc,
    field: &Field,
) -> proc_macro2::TokenStream {
    let method_name = format_ident!("{}", field.method_name());
    let method_body = quote! { it.#method_name() };
    let variants = enum_src
        .variants
        .iter()
        .map(|v| format_ident!("{}", v))
        .collect::<Vec<_>>();
    let ty = field.ty();
    let enum_name = format_ident!("{}", enum_src.name);
    match field {
        Field::Node { cardinality, .. } => match cardinality {
            // Cardinality::Many => {
            //     quote! {
            //         #[inline]
            //         pub fn #method_name(&self) -> AstChildren<#ty> {
            //             match self {
            //                 #(ast::#variants(#variants) => #variants.#method_name(),)*
            //             }
            //         }
            //     }
            // }
            Cardinality::Optional => {
                quote! {
                    #[inline]
                    pub fn #method_name(&self) -> Option<#ty> {
                        match self {
                            #(#enum_name::#variants(it) => #method_body),*
                        }
                    }
                }
            }
            Cardinality::Required => {
                quote! {
                    #[inline]
                    pub fn #method_name(&self) -> #ty {
                        match self {
                            #(#enum_name::#variants(it) => #method_body),*
                        }
                    }
                }
            }
            _ => quote! {},
        },
        Field::Token { name, cardinality } => {
            let token: proc_macro2::TokenStream = name.parse().unwrap();
            let token_kind = quote! { T![#token] };
            match cardinality {
                Cardinality::Optional => {
                    quote! {
                        #[inline]
                        pub fn #method_name(&self) -> Option<#ty> {
                            #(#enum_name::#variants(it) => #method_body),*
                        }
                    }
                }
                _ => quote! {},
            }
        }
    }
}
