use crate::RootDatabase;
use lang::Semantics;
use lang::node_ext::call_ext;
use lang::node_ext::call_ext::CalleeKind;
use lang::types::ty::Ty;
use std::collections::HashSet;
use syntax::algo::ancestors_at_offset;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, NodeOrToken, SyntaxNode, SyntaxToken, T, TextSize, algo, ast};

#[derive(Debug)]
pub struct ActiveParameterInfo {
    pub ty: Option<Ty>,
    pub src: Option<InFile<ast::Param>>,
}

impl ActiveParameterInfo {
    /// Returns information about the call argument this token is part of.
    pub fn at_offset(
        sema: &Semantics<'_, RootDatabase>,
        original_file: &SyntaxNode,
        offset: TextSize,
    ) -> Option<Self> {
        let (any_call_expr, active_parameter) = call_expr_for_offset(original_file, offset)?;

        let any_call_expr = algo::find_node_at_offset::<ast::AnyCallExpr>(
            original_file,
            any_call_expr.syntax().text_range().start(),
        )?;
        let any_call_expr = sema.wrap_node_infile(any_call_expr);

        let msl = any_call_expr.value.syntax().is_msl_context();
        let idx = active_parameter?;

        let (callee_file_id, callee_kind) = call_ext::callee_kind(sema, &any_call_expr)?.unpack();
        match callee_kind {
            CalleeKind::Function(any_fun) => {
                let mut params = any_fun.params();
                if idx >= params.len() {
                    return None;
                }
                let param = params.swap_remove(idx);
                let ty = param
                    .type_()
                    .map(|it| sema.lower_type(it.in_file(callee_file_id), msl));
                Some(ActiveParameterInfo {
                    ty,
                    src: Some(param.in_file(callee_file_id)),
                })
            }
            // todo:
            _ => None,
        }
    }

    pub fn ident(&self) -> Option<ast::Name> {
        let param = self.src.as_ref()?;
        let ident_pat = param.value.ident_pat()?;
        ident_pat.name()
    }
}

pub fn call_expr_for_offset(
    file: &SyntaxNode,
    offset: TextSize,
) -> Option<(ast::AnyCallExpr, Option<usize>)> {
    // Find the calling expression and its NameRef
    let call_expr = ancestors_at_offset(file, offset)
        .filter_map(ast::AnyCallExpr::cast)
        .find(|it| {
            it.value_arg_list()
                .is_some_and(|it| it.syntax().text_range().contains(offset))
        })?;
    let active_param = active_parameter_at_offset(&call_expr, offset);
    Some((call_expr, active_param))
}

pub fn call_expr_for_arg_list(
    arg_list: InFile<ast::ValueArgList>,
    at_offset: TextSize,
) -> Option<(InFile<ast::AnyCallExpr>, Option<usize>)> {
    let (file_id, arg_list) = arg_list.unpack();

    debug_assert!(arg_list.syntax().text_range().contains(at_offset));
    let call_expr = arg_list.syntax().parent().and_then(ast::AnyCallExpr::cast)?;

    let active_param = active_parameter_at_offset(&call_expr, at_offset);
    Some((call_expr.in_file(file_id), active_param))
}

fn active_parameter_at_offset(callable: &ast::AnyCallExpr, at_offset: TextSize) -> Option<usize> {
    let active_param = callable
        .value_arg_list()
        .map(|arg_list| active_param(arg_list.syntax(), at_offset));
    active_param
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

    let active_param = active_param(type_arg_list.syntax(), token.text_range().start());
    Some((generic_item, active_param))
}

pub fn fields_owner_for_struct_lit(
    sema: &Semantics<'_, RootDatabase>,
    struct_lit: InFile<ast::StructLit>,
    offset: TextSize,
) -> Option<(InFile<ast::FieldsOwner>, Option<String>)> {
    let active_lit_field_idx = struct_lit
        .value
        .struct_lit_field_list()
        .map(|list| active_param(list.syntax(), offset))?;

    let lit_fields = struct_lit.value.clone().fields();
    let active_lit_field = lit_fields.get(active_lit_field_idx).and_then(|it| it.name_ref());

    let fields_owner = sema.resolve_to_element::<ast::FieldsOwner>(struct_lit.map(|it| it.path()))?;

    let active_field_name = match active_lit_field {
        Some(name_ref) => Some(name_ref.as_string()),
        None => {
            // compute next field skipping all filled fields
            let all_field_names = fields_owner.value.named_field_names();
            let provided_field_names = lit_fields
                .iter()
                .filter_map(|it| it.field_name())
                .collect::<HashSet<_>>();

            let mut next_field_name: Option<String> = None;
            for field_name in all_field_names {
                if !provided_field_names.contains(&field_name) {
                    next_field_name = Some(field_name);
                    break;
                }
            }
            next_field_name
        }
    };

    Some((fields_owner, active_field_name))
}

fn active_param(list_node: &SyntaxNode, offset: TextSize) -> usize {
    list_node
        .children_with_tokens()
        .filter_map(NodeOrToken::into_token)
        .filter(|t| t.kind() == T![,])
        .take_while(|t| t.text_range().start() <= offset)
        .count()
}
