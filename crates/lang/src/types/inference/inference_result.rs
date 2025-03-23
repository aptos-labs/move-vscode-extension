use crate::db::HirDatabase;
use crate::loc;
use crate::loc::{SyntaxLocFileExt, SyntaxLocNodeExt};
use crate::nameres::scope::{ScopeEntry, ScopeEntryListExt};
use crate::types::inference::InferenceCtx;
use crate::types::ty::Ty;
use std::collections::HashMap;
use syntax::ast::ReferenceElement;
use syntax::{ast, AstNode};
use vfs::FileId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferenceResult {
    file_id: FileId,

    expr_types: HashMap<loc::SyntaxLoc, Ty>,

    resolved_paths: HashMap<loc::SyntaxLoc, Vec<ScopeEntry>>,
    resolved_method_calls: HashMap<loc::SyntaxLoc, Option<ScopeEntry>>,
}

impl InferenceResult {
    pub fn from_ctx(ctx: InferenceCtx) -> Self {
        let expr_types = ctx
            .expr_types
            .clone()
            .into_iter()
            .map(|(expr, ty)| {
                let res_ty = ctx.fully_resolve_vars(ty);
                let expr_loc = expr.loc(ctx.file_id);
                (expr_loc, res_ty)
            })
            .collect();

        let resolved_paths = ctx
            .resolved_paths
            .into_iter()
            .map(|(path, entries)| (path.loc(ctx.file_id), entries))
            .collect();

        let resolved_method_calls = ctx
            .resolved_method_calls
            .into_iter()
            .map(|(method_call, opt_entry)| (method_call.loc(ctx.file_id), opt_entry))
            .collect();

        InferenceResult {
            file_id: ctx.file_id,
            expr_types,
            resolved_paths,
            resolved_method_calls,
        }
    }

    pub fn get_expr_type(&self, expr: &ast::Expr) -> Option<Ty> {
        let expr_loc = expr.loc(self.file_id);
        self.expr_types.get(&expr_loc).map(|it| it.to_owned())
    }

    pub fn resolve_method_or_path(&self, reference: ast::MethodOrPath) -> Option<ScopeEntry> {
        use syntax::SyntaxKind::*;

        match reference.syntax().kind() {
            METHOD_CALL_EXPR => {
                let method_call_expr = reference.cast_into::<ast::MethodCallExpr>().unwrap();
                self.get_resolved_method_call(&method_call_expr)
            }
            PATH => {
                let path = reference.cast_into::<ast::Path>().unwrap();
                self.get_resolved_path(&path)
            }
            _ => None,
        }
    }

    fn get_resolved_path(&self, path: &ast::Path) -> Option<ScopeEntry> {
        let loc = path.loc(self.file_id);
        self.resolved_paths
            .get(&loc)
            .and_then(|entries| entries.clone().single_or_none())
    }

    fn get_resolved_method_call(&self, method_call_expr: &ast::MethodCallExpr) -> Option<ScopeEntry> {
        let loc = method_call_expr.loc(self.file_id);
        self.resolved_method_calls
            .get(&loc)
            .and_then(|method| method.to_owned())
    }
}
