// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use lang::loc::SyntaxLocFileExt;
use lang::nameres::fq_named_element::ItemFQNameOwner;
use lang::types::fold::TypeFoldable;
use lang::types::has_type_params_ext::GenericItemExt;
use lang::types::ty::ty_callable::TyCallableKind;
use syntax::SyntaxKind::*;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{FileRange, InFile, InFileExt};
use syntax::{AstNode, ast};

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn missing_type_arguments(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    method_or_path: InFile<ast::MethodOrPath>,
) -> Option<()> {
    let (file_id, method_or_path) = method_or_path.unpack();

    let path_name_ref = method_or_path.name_ref()?;
    let type_arg_list = method_or_path.type_arg_list().map(|it| it.in_file(file_id));
    let type_args = method_or_path.type_args();

    let actual_count = type_args.len();
    if let ast::MethodOrPath::Path(path) = &method_or_path
        && path_name_ref.as_string() == "vector"
    {
        // check whether it's a `vector<>` type instantiation
        // and it has a single type argument for the element type
        let root_path = path.root_path();
        // relevant only in type position
        if root_path.is_local() && root_path.root_parent_kind() == Some(PATH_TYPE) {
            if actual_count != 1 {
                acc.push(new_type_arguments_mismatch(
                    "vector",
                    1,
                    actual_count,
                    FileRange {
                        file_id,
                        range: method_or_path.syntax().text_range(),
                    },
                ));
                return Some(());
            }
        }
    }

    let parent = method_or_path.syntax().parent()?;
    let item = ctx
        .sema
        .resolve_to_element::<ast::GenericElement>(method_or_path.reference().in_file(file_id))?;
    let expected_count = item.ty_type_params().len();
    let item_label = item.fq_name(ctx.sema.db)?.fq_identifier_text();

    match (item.kind(), parent.kind()) {
        (STRUCT, PATH_TYPE) => {
            if parent.has_ancestor_strict::<ast::Acquires>() {
                return None;
            }
            if let Some(type_arg_list) = type_arg_list {
                check_type_arg_list(acc, item_label, expected_count, actual_count, type_arg_list);
            } else {
                if expected_count != 0 {
                    acc.push(new_type_arguments_mismatch(
                        item_label,
                        expected_count,
                        actual_count,
                        method_or_path.in_file(file_id).file_range(),
                    ))
                }
            }
        }
        (STRUCT, STRUCT_LIT) => {
            if let Some(type_arg_list) = type_arg_list {
                // if any type param is passed, inference is disabled, so check fully
                check_type_arg_list(acc, item_label, expected_count, actual_count, type_arg_list);
            } else {
                // todo: check whether type are inferrable from fields
            }
        }
        (FUN | SPEC_FUN | SPEC_INLINE_FUN, _) => {
            let call_expr: ast::AnyCallExpr = match &method_or_path {
                ast::MethodOrPath::Path(path) => path
                    .path_expr()?
                    .syntax()
                    .parent_of_type::<ast::CallExpr>()?
                    .clone()
                    .into(),
                ast::MethodOrPath::MethodCallExpr(method_call_expr) => method_call_expr.clone().into(),
            };
            if let Some(type_arg_list) = type_arg_list {
                check_type_arg_list(acc, item_label, expected_count, actual_count, type_arg_list)
            } else {
                if !ctx.config.needs_type_annotation {
                    return None;
                }
                let call_expr = call_expr.in_file(file_id);
                let msl = method_or_path.syntax().is_msl_context();
                let inference = ctx.sema.inference(&call_expr, msl)?;
                if inference.has_type_error_inside_range(call_expr.value.syntax().text_range()) {
                    return None;
                }
                let callable_ty = inference.get_call_expr_type(&call_expr.loc())?;
                if call_expr.value.n_provided_args() < callable_ty.param_types.len() {
                    // no error if missing value arguments
                    return None;
                }
                if let TyCallableKind::Named(subst, _) = &callable_ty.kind {
                    if subst.has_ty_infer() {
                        acc.push(needs_type_annotation(path_name_ref.in_file(file_id).file_range()));
                    }
                }
            }
        }
        (SCHEMA, SCHEMA_LIT) => {
            if let Some(type_arg_list) = type_arg_list {
                // if any type param is passed, inference is disabled, so check fully
                check_type_arg_list(acc, item_label, expected_count, actual_count, type_arg_list);
            } else {
                // todo: check whether type are inferrable from fields
            }
        }
        _ => (),
    }

    Some(())
}

fn check_type_arg_list(
    acc: &mut Vec<Diagnostic>,
    item_label: String,
    expected_count: usize,
    actual_count: usize,
    type_args_list: InFile<ast::TypeArgList>,
) {
    if expected_count == 0 {
        acc.push(no_type_arguments_expected(
            item_label,
            type_args_list.file_range(),
        ));
    } else if actual_count < expected_count {
        acc.push(new_type_arguments_mismatch(
            item_label,
            expected_count,
            actual_count,
            type_args_list.file_range(),
        ))
    } else if actual_count > expected_count {
        for type_arg in type_args_list
            .as_ref()
            .flat_map(|it| it.type_args())
            .into_iter()
            .skip(expected_count)
        {
            acc.push(new_type_arguments_mismatch(
                &item_label,
                expected_count,
                actual_count,
                type_arg.file_range(),
            ))
        }
    }
}

fn new_type_arguments_mismatch(
    item_label: impl Into<String>,
    expected_count: usize,
    actual_count: usize,
    file_range: FileRange,
) -> Diagnostic {
    let item_label = item_label.into();
    Diagnostic::new(
        DiagnosticCode::Lsp("type-arguments-number-mismatch", Severity::Error),
        format!(
            "Invalid instantiation of '{item_label}'. Expected {expected_count} type argument(s), but got {actual_count}"
        ),
        file_range,
    )
}

fn no_type_arguments_expected(item_label: impl Into<String>, file_range: FileRange) -> Diagnostic {
    let item_label = item_label.into();
    Diagnostic::new(
        DiagnosticCode::Lsp("type-arguments-number-mismatch", Severity::Error),
        format!("No type arguments expected for '{item_label}'"),
        file_range,
    )
}

fn needs_type_annotation(file_range: FileRange) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::Lsp("needs-type-annotation", Severity::Error),
        "Could not infer this type. Try adding a type annotation",
        file_range,
    )
}
