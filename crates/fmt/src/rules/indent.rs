use crate::engine::fmt_model::IndentType;
use crate::rules::spacing::BIN_OPS;
use syntax::SyntaxKind::*;
use syntax::{AstNode as _, SyntaxKind};

/// Compute the indent type for a child given its parent kind and position.
pub(crate) fn get_indent_type(
    parent_kind: SyntaxKind,
    child_kind: SyntaxKind,
    after_l_curly: bool,
) -> IndentType {
    if is_delimiter(child_kind) {
        return IndentType::None;
    }

    match parent_kind {
        // Flat blocks: MODULE, SCRIPT, MODULE_SPEC have L_CURLY/R_CURLY
        // as direct children. Items between braces get indented.
        MODULE | SCRIPT | MODULE_SPEC => {
            if after_l_curly {
                IndentType::Block
            } else {
                IndentType::None
            }
        }
        // ADDRESS_DEF: modules inside are NOT indented.
        ADDRESS_DEF => IndentType::None,

        // Delimited blocks: children between braces/parens get indented.
        BLOCK_EXPR
        | SPEC_BLOCK_EXPR
        | SCHEMA_LIT
        | NAMED_FIELD_LIST
        | VARIANT_LIST
        | MATCH_ARM_LIST
        | STRUCT_LIT_FIELD_LIST
        | SCHEMA_LIT_FIELD_LIST
        | USE_GROUP
        | VALUE_ARG_LIST
        | PARAM_LIST
        | ITEM_SPEC_PARAM_LIST
        | PAREN_EXPR => IndentType::Block,

        LET_STMT if syntax::ast::Expr::can_cast(child_kind) => IndentType::Continuation,

        // after `=` in let stmt, const
        INITIALIZER if syntax::ast::Expr::can_cast(child_kind) => IndentType::Continuation,

        SPEC_PREDICATE_STMT if syntax::ast::Expr::can_cast(child_kind) => IndentType::Continuation,

        ABORTS_IF_STMT if child_kind == ABORTS_IF_WITH => IndentType::Continuation,

        BIN_EXPR if syntax::ast::Expr::can_cast(child_kind) => IndentType::Continuation,

        BIN_EXPR if is_binary_op(child_kind) => IndentType::Continuation,

        IMPLY_INCLUDE_EXPR if child_kind == IMPLIES => IndentType::Continuation,

        // Pragma continuation items get one extra indent.
        PRAGMA_STMT if child_kind == PRAGMA_ATTR_ITEM => IndentType::Continuation,

        _ => IndentType::None,
    }
}

fn is_binary_op(kind: SyntaxKind) -> bool {
    BIN_OPS.contains(kind)
}

fn is_delimiter(kind: SyntaxKind) -> bool {
    matches!(kind, L_CURLY | R_CURLY | L_PAREN | R_PAREN | L_BRACK | R_BRACK)
}
