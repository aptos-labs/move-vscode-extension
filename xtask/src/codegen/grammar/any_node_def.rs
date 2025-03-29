use crate::codegen::grammar::ast_src::AstNodeSrc;
use crate::codegen::grammar::{find_common_traits, to_upper_snake_case};
use quote::{format_ident, quote};

#[derive(Debug, Clone)]
pub struct AnyNodeDefSrc {
    pub trait_name: String,
    pub common_traits: Vec<String>,
    pub from_impls: Vec<String>,
    pub kinds: Vec<String>,
}

pub(crate) fn extract_any_node_def(trait_name: &str, nodes: Vec<&AstNodeSrc>) -> AnyNodeDefSrc {
    let any_trait_name = format_ident!("Any{}", trait_name);

    let common_traits = find_common_traits(nodes.clone())
        .into_iter()
        .filter(|common_trait| common_trait != trait_name)
        .collect::<Vec<_>>();

    let impl_common_traits = common_traits
        .iter()
        .map(|common_trait| {
            let common_trait = format_ident!("{}", common_trait);
            quote! { impl ast::#common_trait for #any_trait_name {} }
        })
        .collect::<Vec<_>>();

    let kinds = nodes
        .iter()
        .map(|name| to_upper_snake_case(&name.name.to_string()))
        .collect();
    let node_names = nodes.iter().map(|node| node.name.clone()).collect();

    AnyNodeDefSrc {
        trait_name: trait_name.to_string(),
        common_traits,
        from_impls: node_names,
        kinds,
    }
}

pub(crate) fn find_node_defs_with_trait(
    trait_name: &String,
    node_defs: Vec<AnyNodeDefSrc>,
) -> Vec<AnyNodeDefSrc> {
    node_defs
        .into_iter()
        .filter(|node_def| node_def.common_traits.contains(trait_name))
        .collect()
}

pub(crate) fn generate_any_node_def(
    any_node_def: AnyNodeDefSrc,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let trait_name = format_ident!("{}", any_node_def.trait_name);
    let any_trait_name = format_ident!("Any{}", trait_name);

    let impl_common_traits = any_node_def
        .common_traits
        .iter()
        .map(|common_trait| {
            let common_trait = format_ident!("{}", common_trait);
            quote! { impl ast::#common_trait for #any_trait_name {} }
        })
        .collect::<Vec<_>>();

    let kinds: Vec<_> = any_node_def
        .kinds
        .iter()
        .cloned()
        .map(|kind| format_ident!("{}", kind))
        .collect();
    let nodes = any_node_def
        .from_impls
        .iter()
        .cloned()
        .map(|node| format_ident!("{}", node));

    (
        quote! {
            #[pretty_doc_comment_placeholder_workaround]
            #[derive(Debug, Clone, PartialEq, Eq, Hash)]
            pub struct #any_trait_name {
                pub(crate) syntax: SyntaxNode,
            }
            impl ast::#trait_name for #any_trait_name {}
            #(#impl_common_traits)*
        },
        quote! {
            impl #any_trait_name {
                #[inline]
                pub fn new<T: ast::#trait_name>(node: T) -> #any_trait_name {
                    #any_trait_name {
                        syntax: node.syntax().clone()
                    }
                }
                #[inline]
                pub fn cast_from<T: ast::#trait_name>(t: T) -> #any_trait_name {
                    #any_trait_name::cast(t.syntax().to_owned()).expect("required by code generator")
                }
                #[inline]
                pub fn cast_into<T: ast::#trait_name>(&self) -> Option<T> {
                    T::cast(self.syntax().to_owned())
                }
            }
            impl AstNode for #any_trait_name {
                #[inline]
                fn can_cast(kind: SyntaxKind) -> bool {
                    matches!(kind, #(#kinds)|*)
                }
                #[inline]
                fn cast(syntax: SyntaxNode) -> Option<Self> {
                    Self::can_cast(syntax.kind()).then_some(#any_trait_name { syntax })
                }
                #[inline]
                fn syntax(&self) -> &SyntaxNode {
                    &self.syntax
                }
            }

            #(
                impl From<#nodes> for #any_trait_name {
                    #[inline]
                    fn from(node: #nodes) -> #any_trait_name {
                        #any_trait_name { syntax: node.syntax }
                    }
                }
            )*
        },
    )
}
