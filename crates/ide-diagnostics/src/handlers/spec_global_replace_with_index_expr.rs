// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::DiagnosticsContext;
use crate::diagnostic::Diagnostic;
use ide_db::assist_context::LocalAssists;
use lang::nameres::scope::VecExt;
use lang::node_ext::ModuleLangExt;
use lang::types::abilities::Ability;
use lang::types::ty::Ty;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::ast::syntax_factory::SyntaxFactory;
use syntax::files::{FileRange, InFile, InFileExt};
use syntax::{AstNode, ast};

const DIAGNOSTIC_ID: &str = "spec-global-replace-with-index-expr";

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn spec_global_replace_with_index_expr(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    call_expr: InFile<ast::CallExpr>,
) -> Option<()> {
    let (file_id, call_expr) = call_expr.unpack();
    if !call_expr.syntax().is_msl_context() {
        return None;
    }

    let call_path = call_expr.path()?;
    let spec_fun = ctx
        .sema
        .resolve_to_element::<ast::SpecFun>(call_path.reference().in_file(file_id))?;
    let module = ctx.sema.fun_module(spec_fun)?;
    // only 0x0::builtins is valid
    if !module.value.is_builtins() {
        return None;
    }

    // single type arg only
    let arg_type = call_path.type_args().single_or_none()?.type_()?;
    // has to be a type with `key` ability
    let type_abilities = ctx
        .sema
        .lower_type(arg_type.in_file(file_id), true)
        .abilities(ctx.sema.db)?;
    if !type_abilities.contains(&Ability::Key) {
        return None;
    }

    let addr_expr = call_expr.arg_exprs().single_or_none()??;
    let addr_expr_ty = ctx.sema.get_expr_type(&addr_expr.in_file(file_id))?;
    if !matches!(addr_expr_ty, Ty::Address) {
        return None;
    }

    let node_range = FileRange {
        file_id,
        range: call_expr.syntax().text_range(),
    };
    acc.push(
        Diagnostic::weak_warning(DIAGNOSTIC_ID, "Replace with resource index expr", node_range)
            .with_local_fixes(fixes(ctx, call_expr.in_file(file_id), node_range)),
    );

    Some(())
}

fn fixes(
    ctx: &DiagnosticsContext<'_>,
    call_expr: InFile<ast::CallExpr>,
    diagnostic_range: FileRange,
) -> Option<LocalAssists> {
    let mut assists = ctx.local_assists_for_node(call_expr.as_ref())?;
    assists.add_fix_fallible(
        DIAGNOSTIC_ID,
        "Replace with resource index expr",
        diagnostic_range.range,
        |editor| {
            let (_, call_expr) = call_expr.unpack();
            let call_path = call_expr.path()?;
            let resource_path = call_path
                .type_args()
                .single_or_none()?
                .type_()?
                .path_type()?
                .path();

            let make = SyntaxFactory::new();

            let resource_path_expr = make.path_expr(resource_path);
            let addr_expr = call_expr.arg_exprs().single_or_none()??;
            let resource_index_expr = make.index_expr(resource_path_expr.into(), addr_expr);
            editor.replace(call_expr.syntax(), resource_index_expr.syntax());

            Some(())
        },
    );
    Some(assists)
}
