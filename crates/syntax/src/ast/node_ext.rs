mod address_ref;
mod any_fun;
mod assert_macro_expr;
pub mod attr;
pub mod bin_expr;
mod block_expr;
mod borrow_expr;
mod call_expr;
mod const_;
mod enum_;
mod expr;
mod field_ref;
mod fields_owner;
pub mod fun;
mod ident_pat;
mod if_expr;
pub mod index_expr;
mod is_expr;
mod lambda_expr;
mod lambda_type;
pub mod literal;
mod match_expr;
mod method_call_expr;
mod method_or_path;
mod module;
pub mod move_syntax_node;
pub mod name_ref;
pub mod named_field;
mod param;
mod pat;
pub mod path;
mod range_expr;
mod ref_type;
mod schema;
mod source_file;
mod struct_lit;
mod struct_lit_field;
mod struct_or_enum;
mod struct_pat;
pub mod struct_pat_field;
pub mod syntax_node;
pub mod type_;
mod vector_lit_expr;
pub mod visibility;
mod quant_expr;

use crate::token_text::TokenText;
use crate::{ast, AstNode, AstToken, SyntaxNode};
use parser::SyntaxKind;
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
