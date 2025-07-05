use crate::RootDatabase;
use lang::Semantics;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, NodeOrToken, SyntaxToken, T, TextSize, ast};

pub fn callable_for_arg_list(
    arg_list: InFile<ast::ValueArgList>,
    at_offset: TextSize,
) -> Option<(InFile<ast::AnyCallExpr>, Option<usize>)> {
    let (file_id, arg_list) = arg_list.unpack();

    debug_assert!(arg_list.syntax().text_range().contains(at_offset));
    let callable = arg_list.syntax().parent().and_then(ast::AnyCallExpr::cast)?;
    let active_param = callable.value_arg_list().map(|arg_list| {
        arg_list
            .syntax()
            .children_with_tokens()
            .filter_map(NodeOrToken::into_token)
            .filter(|t| t.kind() == T![,])
            .take_while(|t| t.text_range().start() <= at_offset)
            .count()
    });
    Some((callable.in_file(file_id), active_param))
}

pub fn generic_item_for_type_arg_list(
    sema: &Semantics<'_, RootDatabase>,
    type_arg_list: InFile<ast::TypeArgList>,
    token: &SyntaxToken,
) -> Option<(InFile<ast::GenericElement>, usize)> {
    let (file_id, type_arg_list) = type_arg_list.unpack();

    let method_or_path = type_arg_list.method_or_path()?;
    let generic_item =
        sema.resolve_to_element::<ast::GenericElement>(method_or_path.in_file(file_id))?;

    let active_param = type_arg_list
        .syntax()
        .children_with_tokens()
        .filter_map(NodeOrToken::into_token)
        .filter(|t| t.kind() == T![,])
        .take_while(|t| t.text_range().start() <= token.text_range().start())
        .count();

    Some((generic_item, active_param))
}
