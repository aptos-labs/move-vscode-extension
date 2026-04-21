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
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::ast::syntax_factory::SyntaxFactory;
use syntax::files::{FileRange, InFile, InFileExt};

const DIAGNOSTIC_ID: &str = "replace-with-resource-index-expr";

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn replace_with_resource_index_expr(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    call_expr: InFile<ast::CallExpr>,
) -> Option<()> {
    let (file_id, call_expr) = call_expr.unpack();

    // check that type parameter has `key`
    let resource_type = call_expr.path()?.type_args().single_or_none()?.type_()?;
    let resource_ty = ctx.sema.lower_type(resource_type.clone().in_file(file_id), false);
    if !resource_ty.abilities(ctx.sema.db)?.contains(&Ability::Key) {
        return None;
    }

    // check whether it's 0x0::builtins::borrow_global
    let (fun_file_id, fun) = ctx
        .sema
        .resolve_to_element::<ast::Fun>(call_expr.path()?.reference().in_file(file_id))?
        .unpack();
    if !ctx.sema.is_builtins_file(fun_file_id) {
        return None;
    }

    let fun_name = fun.name()?;
    if !fun_name.text().starts_with("borrow_global") {
        return None;
    }

    // check that first param is address
    let resource_path = resource_type.path_type()?.path();
    let addr_expr = call_expr.arg_exprs().single_or_none()??;
    let addr_expr_ty = ctx.sema.get_expr_type(&addr_expr.clone().in_file(file_id))?;
    if !matches!(addr_expr_ty, Ty::Address) {
        return None;
    }

    let call_expr_parent = call_expr.syntax().parent()?;
    // borrow_global<T>().field
    let borrow_ctx = if call_expr_parent.is::<ast::MethodOrDotExpr>() {
        BorrowCtx::Dotted
    } else if call_expr_parent.is_msl_context() {
        BorrowCtx::Spec
    } else {
        match fun_name.text().as_str() {
            "borrow_global" => BorrowCtx::Expr { is_mut: false },
            "borrow_global_mut" => BorrowCtx::Expr { is_mut: true },
            _ => {
                return None;
            }
        }
    };

    let node_range = FileRange {
        file_id,
        range: call_expr.syntax().text_range(),
    };
    acc.push(
        Diagnostic::weak_warning(DIAGNOSTIC_ID, "Replace with resource index expr", node_range)
            .with_local_fixes(fixes(
                ctx,
                call_expr.in_file(file_id),
                node_range,
                resource_path,
                addr_expr,
                borrow_ctx,
            )),
    );

    None
}

fn fixes(
    ctx: &DiagnosticsContext<'_>,
    call_expr: InFile<ast::CallExpr>,
    diagnostic_range: FileRange,
    resource_path: ast::Path,
    addr_expr: ast::Expr,
    borrow_ctx: BorrowCtx,
) -> Option<LocalAssists> {
    let mut assists = ctx.local_assists_for_node(call_expr.as_ref())?;
    assists.add_fix(
        DIAGNOSTIC_ID,
        "Replace with resource index expr",
        diagnostic_range.range,
        |editor| {
            let make = SyntaxFactory::new();
            let resource_path_expr = make.path_expr(resource_path);
            let resource_index_expr = make.index_expr(resource_path_expr.into(), addr_expr);
            match borrow_ctx {
                BorrowCtx::Dotted => {
                    editor.replace(call_expr.value.syntax(), resource_index_expr.syntax());
                }
                BorrowCtx::Spec => {
                    editor.replace(call_expr.value.syntax(), resource_index_expr.syntax());
                }
                BorrowCtx::Expr { is_mut } => {
                    let borrow_expr = make.borrow_expr(resource_index_expr.into(), is_mut);
                    editor.replace(call_expr.value.syntax(), borrow_expr.syntax());
                }
            }
        },
    );
    Some(assists)
}

enum BorrowCtx {
    Expr { is_mut: bool },
    Dotted,
    Spec,
}
