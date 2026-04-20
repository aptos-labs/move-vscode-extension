// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::DiagnosticsContext;
use crate::diagnostic::Diagnostic;
use ide_db::assist_context::LocalAssists;
use lang::nameres::scope::VecExt;
use lang::types::abilities::Ability;
use lang::types::ty::Ty;
use syntax::AstNode;
use syntax::ast;
use syntax::ast::syntax_factory::SyntaxFactory;
use syntax::files::{FileRange, InFile, InFileExt};

const DIAGNOSTIC_ID: &str = "replace-with-resource-index-expr";

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn replace_with_resource_index_expr(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    dot_expr: InFile<ast::DotExpr>,
) -> Option<()> {
    let (file_id, dot_expr) = dot_expr.unpack();

    let receiver_call_expr = dot_expr.receiver_expr().call_expr()?;
    // check that type parameter has `key`
    let resource_type = receiver_call_expr.path()?.type_args().single_or_none()?.type_()?;
    let resource_ty = ctx.sema.lower_type(resource_type.clone().in_file(file_id), false);
    if !resource_ty.abilities(ctx.sema.db)?.contains(&Ability::Key) {
        return None;
    }

    // check whether it's 0x0::builtins::borrow_global
    let (fun_file_id, fun) = ctx
        .sema
        .resolve_to_element::<ast::Fun>(receiver_call_expr.path()?.reference().in_file(file_id))?
        .unpack();
    if !ctx.sema.is_builtins_file(fun_file_id) {
        return None;
    }

    let fun_name = fun.name()?;
    if !fun_name.text().starts_with("borrow_global") {
        return None;
    }

    // // check whether field has `copy`
    // let (field_file_id, field) = ctx
    //     .sema
    //     .resolve_to_element::<ast::NamedField>(dot_expr.reference().in_file(file_id))?
    //     .unpack();
    // let field_ty = ctx.sema.lower_type(field.type_()?.in_file(field_file_id), false);
    // if !field_ty.abilities(ctx.sema.db)?.contains(&Ability::Copy) {
    //     return None;
    // }

    let resource_path = resource_type.path_type()?.path();
    let addr_expr = receiver_call_expr.arg_exprs().single_or_none()??;
    let addr_expr_ty = ctx.sema.get_expr_type(&addr_expr.clone().in_file(file_id))?;
    if !matches!(addr_expr_ty, Ty::Address) {
        return None;
    }
    let node_range = FileRange {
        file_id,
        range: dot_expr.syntax().text_range(),
    };
    acc.push(
        Diagnostic::weak_warning(DIAGNOSTIC_ID, "Replace with resource index expr", node_range)
            .with_local_fixes(fixes(
                ctx,
                dot_expr.in_file(file_id),
                node_range,
                resource_path,
                addr_expr,
            )),
    );

    None
}

fn fixes(
    ctx: &DiagnosticsContext<'_>,
    dot_expr: InFile<ast::DotExpr>,
    diagnostic_range: FileRange,
    resource_path: ast::Path,
    addr_expr: ast::Expr,
) -> Option<LocalAssists> {
    let mut assists = ctx.local_assists_for_node(dot_expr.as_ref())?;
    assists.add_fix(
        DIAGNOSTIC_ID,
        "Replace with resource index expr",
        diagnostic_range.range,
        |editor| {
            let make = SyntaxFactory::new();
            let resource_path_expr = make.path_expr(resource_path);
            let resource_index_expr = make.index_expr(resource_path_expr.into(), addr_expr);
            let receiver_expr = dot_expr.value.receiver_expr();
            editor.replace(receiver_expr.syntax(), resource_index_expr.syntax());
        },
    );
    Some(assists)
}
