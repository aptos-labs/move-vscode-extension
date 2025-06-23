mod address_ref;
mod and_include_expr;
mod any_fun;
mod any_reference_element;
mod apply_schema;
mod assert_macro_expr;
pub mod attr;
mod axiom_stmt;
pub mod bin_expr;
mod block_expr;
mod block_or_inline_expr;
mod borrow_expr;
mod call_expr;
mod const_;
mod enum_;
mod expr;
mod field_list;
mod fields_owner;
pub mod fun;
mod generic_element;
mod generic_spec_stmt;
mod ident_pat;
mod ident_pat_owner;
mod if_else_include_expr;
mod if_expr;
mod include_expr;
pub mod index_expr;
mod invariant_stmt;
mod is_expr;
mod item_spec;
mod item_spec_ref;
mod item_spec_type_param;
mod lambda_expr;
mod lambda_param;
mod lambda_type;
mod let_stmt;
pub mod literal;
mod loop_label;
mod match_expr;
mod method_call_expr;
mod method_or_path;
mod module;
mod module_spec;
pub mod move_syntax_node;
pub mod name_ref;
pub mod named_field;
mod param;
mod pat;
pub mod path;
mod path_expr;
mod quant_bindings_owner;
mod quant_expr;
mod range_expr;
mod ref_type;
mod schema;
mod schema_field;
mod schema_lit;
mod schema_lit_field;
mod source_file;
pub mod spec_predicate_stmt;
mod struct_;
mod struct_lit;
mod struct_lit_field;
mod struct_or_enum;
mod struct_pat;
pub mod struct_pat_field;
pub mod syntax_element;
pub mod syntax_node;
pub mod syntax_token;
pub mod type_;
mod type_param;
mod value_arg_list;
mod vector_lit_expr;
pub mod visibility;
mod wildcard_pattern;

use crate::parse::SyntaxKind;
use crate::token_text::TokenText;
use crate::{ast, AstNode, AstToken, SyntaxNode};
use rowan::{GreenNodeData, GreenTokenData, NodeOrToken};
use std::borrow::Cow;

impl ast::Name {
    pub fn text(&self) -> TokenText<'_> {
        text_of_first_token(self.syntax())
    }
}

fn text_of_first_token(node: &SyntaxNode) -> TokenText<'_> {
    fn first_token(green_ref: &GreenNodeData) -> &GreenTokenData {
        green_ref
            .children()
            .next()
            .and_then(NodeOrToken::into_token)
            .unwrap()
    }

    match node.green() {
        Cow::Borrowed(green_ref) => TokenText::borrowed(first_token(green_ref).text()),
        Cow::Owned(green) => TokenText::owned(first_token(&green).to_owned()),
    }
}

impl ast::PathSegment {
    pub fn parent_path(&self) -> ast::Path {
        self.syntax()
            .parent()
            .and_then(ast::Path::cast)
            .expect("segments are always nested in paths")
    }
}
