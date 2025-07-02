//! This module implements syntax validation that the parser doesn't handle.
//!
//! A failed validation emits a diagnostic.

use crate::{SyntaxError, SyntaxNode, match_ast};

pub(crate) fn validate(root: &SyntaxNode, errors: &mut Vec<SyntaxError>) {
    let _p = tracing::info_span!("parser::validate").entered();
    // FIXME:
    // * Add unescape validation of raw string literals and raw byte string literals
    // * Add validation of doc comments are being attached to nodes

    for node in root.descendants() {
        match_ast! {
            match node {
                // ast::Literal(it) => validate_literal(it, errors),
                // ast::Const(it) => validate_const(it, errors),
                // ast::BlockExpr(it) => block::validate_block_expr(it, errors),
                // ast::FieldExpr(it) => validate_numeric_name(it.name_ref(), errors),
                // ast::RecordExprField(it) => validate_numeric_name(it.name_ref(), errors),
                // ast::Visibility(it) => validate_visibility(it, errors),
                // ast::RangeExpr(it) => validate_range_expr(it, errors),
                // ast::PathSegment(it) => validate_path_keywords(it, errors),
                // ast::RefType(it) => validate_trait_object_ref_ty(it, errors),
                // ast::PtrType(it) => validate_trait_object_ptr_ty(it, errors),
                // ast::FnPtrType(it) => validate_trait_object_fn_ptr_ret_ty(it, errors),
                // ast::MacroRules(it) => validate_macro_rules(it, errors),
                // ast::LetExpr(it) => validate_let_expr(it, errors),
                _ => (),
            }
        }
    }
}
