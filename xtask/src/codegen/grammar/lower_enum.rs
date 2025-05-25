use crate::codegen::grammar::ast_src::{Cardinality, Field, get_required_fields};
use crate::codegen::grammar::lower_rule;
use quote::{format_ident, quote};
use std::collections::HashSet;
use std::env::var;
use std::ops::Deref;
use ungrammar::{Grammar, Rule};

#[derive(Debug)]
pub(crate) struct AstEnumSrc {
    pub(crate) doc: Vec<String>,
    pub(crate) name: String,
    pub(crate) traits: Vec<String>,
    pub(crate) variants: Vec<String>,
    pub(crate) common_fields: Vec<Field>,
}

pub(super) fn lower_enum(grammar: &Grammar, node_name: &str, rule: &Rule) -> Option<AstEnumSrc> {
    // exclude FieldRef as it should be a struct
    if node_name == "FieldRef" {
        return None;
    }
    let alternatives = match rule {
        Rule::Alt(it) => it,
        _ => return None,
    };
    let mut variants = Vec::new();
    let mut common_fields = None;
    for alternative in alternatives {
        match alternative {
            Rule::Node(it) => {
                let node_data = &grammar[*it];
                let variant_name = node_data.name.clone();
                let required_fields = get_required_fields(&variant_name);
                let mut variant_fields = vec![];
                if let Rule::Seq(rules) = &node_data.rule {
                    for child_rule in rules {
                        match child_rule {
                            Rule::Opt(rule) => {
                                if let Rule::Node(_) = rule.deref() {
                                    lower_rule(
                                        &mut variant_fields,
                                        grammar,
                                        None,
                                        child_rule,
                                        required_fields,
                                    );
                                }
                            }
                            Rule::Node(node) => {
                                lower_rule(
                                    &mut variant_fields,
                                    grammar,
                                    None,
                                    child_rule,
                                    required_fields,
                                );
                            }
                            Rule::Labeled { label, rule } => match rule.deref() {
                                Rule::Node(_) => {
                                    lower_rule(
                                        &mut variant_fields,
                                        grammar,
                                        Some(label),
                                        rule,
                                        required_fields,
                                    );
                                }
                                Rule::Opt(rule) => {
                                    lower_rule(
                                        &mut variant_fields,
                                        grammar,
                                        Some(label),
                                        rule,
                                        required_fields,
                                    );
                                }
                                _ => (),
                            },
                            _ => (),
                        }
                    }
                }

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

                variants.push(variant_name)
            }
            Rule::Token(it) if grammar[*it].name == ";" => (),
            _ => return None,
        }
    }
    let common_fields = common_fields.unwrap_or_default();

    let enum_src = AstEnumSrc {
        doc: vec![],
        name: node_name.to_string(),
        traits: vec![],
        variants,
        common_fields,
    };
    Some(enum_src)
}

pub(super) fn generate_field_method_for_enum(
    enum_src: &AstEnumSrc,
    field: &Field,
) -> proc_macro2::TokenStream {
    if field.method_name() == "name" {
        return quote! {};
    }
    let method_name = format_ident!("{}", field.method_name());
    let method_body = match field.method_name().as_str() {
        "name" => {
            quote! { ast::NamedElement::name(it) }
        }
        "type_param_list" => {
            quote! { ast::GenericElement::type_param_list(it) }
        }
        _ => {
            quote! { it.#method_name() }
        }
    };
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
