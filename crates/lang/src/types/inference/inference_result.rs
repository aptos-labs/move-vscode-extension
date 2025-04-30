use crate::loc;
use crate::loc::SyntaxLocNodeExt;
use crate::nameres::scope::{ScopeEntry, VecExt};
use crate::types::inference::InferenceCtx;
use crate::types::ty::Ty;
use crate::types::ty::integer::IntegerKind;
use crate::types::ty::ty_var::TyInfer;
use std::cell::RefCell;
use std::collections::HashMap;
use syntax::{AstNode, ast};
use vfs::FileId;

#[derive(Debug, PartialEq, Eq)]
pub struct InferenceResult {
    file_id: FileId,

    pat_types: HashMap<loc::SyntaxLoc, Ty>,
    expr_types: HashMap<loc::SyntaxLoc, Ty>,

    resolved_paths: HashMap<loc::SyntaxLoc, Vec<ScopeEntry>>,
    resolved_method_calls: HashMap<loc::SyntaxLoc, Option<ScopeEntry>>,
    resolved_fields: HashMap<loc::SyntaxLoc, Option<ScopeEntry>>,
    resolved_ident_pats: HashMap<loc::SyntaxLoc, Option<ScopeEntry>>,
}

impl InferenceResult {
    pub fn from_ctx(mut ctx: InferenceCtx) -> Self {
        Self::unify_remaining_int_vars_into_integer(&mut ctx);

        let pat_types = fully_resolve_map_values(ctx.pat_types.clone(), &ctx);
        let expr_types = fully_resolve_map_values(ctx.expr_types.clone(), &ctx);

        let file_id = ctx.file_id;
        let resolved_paths = keys_into_syntax_loc(ctx.resolved_paths, file_id);
        let resolved_method_calls = keys_into_syntax_loc(ctx.resolved_method_calls, file_id);
        let resolved_fields = keys_into_syntax_loc(ctx.resolved_fields, file_id);
        let resolved_ident_pats = keys_into_syntax_loc(ctx.resolved_ident_pats, file_id);

        InferenceResult {
            file_id: ctx.file_id,
            pat_types,
            expr_types,
            resolved_paths,
            resolved_method_calls,
            resolved_fields,
            resolved_ident_pats,
        }
    }

    fn unify_remaining_int_vars_into_integer(ctx: &mut InferenceCtx) {
        let int_vars = RefCell::new(vec![]);
        {
            for ty in ctx.pat_types.values().chain(ctx.expr_types.values()) {
                ty.deep_visit_ty_infers(|ty_infer| {
                    let resolved_ty_infer = ctx.resolve_ty_infer(ty_infer);
                    if let Ty::Infer(TyInfer::IntVar(int_var)) = resolved_ty_infer {
                        int_vars.borrow_mut().push(int_var);
                    }
                    false
                });
            }
        }
        for int_var in int_vars.into_inner() {
            let _ = ctx.unify_int_var(int_var, Ty::Integer(IntegerKind::Integer));
        }
    }

    pub fn get_pat_type(&self, pat: &ast::Pat) -> Option<Ty> {
        let pat_loc = pat.loc(self.file_id);
        self.pat_types.get(&pat_loc).map(|it| it.to_owned())
    }

    pub fn get_expr_type(&self, expr: &ast::Expr) -> Option<Ty> {
        let expr_loc = expr.loc(self.file_id);
        self.expr_types.get(&expr_loc).map(|it| it.to_owned())
    }

    pub fn get_resolve_method_or_path_entries(
        &self,
        method_or_path: ast::MethodOrPath,
    ) -> Vec<ScopeEntry> {
        match method_or_path {
            ast::MethodOrPath::MethodCallExpr(method_call) => self
                .get_resolved_method_call(&method_call)
                .map(|e| vec![e])
                .unwrap_or_default(),
            ast::MethodOrPath::Path(path) => self.get_resolved_path_entries(&path),
        }
    }

    pub fn get_resolve_method_or_path(&self, method_or_path: ast::MethodOrPath) -> Option<ScopeEntry> {
        self.get_resolve_method_or_path_entries(method_or_path)
            .single_or_none()
    }

    pub fn get_resolved_field(&self, field_ref: &ast::FieldRef) -> Option<ScopeEntry> {
        let loc = field_ref.loc(self.file_id);
        self.resolved_fields.get(&loc).and_then(|field| field.to_owned())
    }

    pub fn get_resolved_ident_pat(&self, ident_pat: &ast::IdentPat) -> Option<ScopeEntry> {
        let loc = ident_pat.loc(self.file_id);
        self.resolved_ident_pats
            .get(&loc)
            .and_then(|ident_pat| ident_pat.to_owned())
    }

    fn get_resolved_path_entries(&self, path: &ast::Path) -> Vec<ScopeEntry> {
        let loc = path.loc(self.file_id);
        self.resolved_paths
            .get(&loc)
            .map(|entries| entries.clone())
            .unwrap_or_default()
        // .and_then(|entries| entries.clone().single_or_none())
    }

    fn get_resolved_method_call(&self, method_call_expr: &ast::MethodCallExpr) -> Option<ScopeEntry> {
        let loc = method_call_expr.loc(self.file_id);
        self.resolved_method_calls
            .get(&loc)
            .and_then(|method| method.to_owned())
    }
}

fn fully_resolve_map_values(
    ty_map: HashMap<impl AstNode, Ty>,
    ctx: &InferenceCtx,
) -> HashMap<loc::SyntaxLoc, Ty> {
    ty_map
        .into_iter()
        .map(|(pat, ty)| {
            let res_ty = ctx.fully_resolve_vars(ty);
            (pat.loc(ctx.file_id), res_ty)
        })
        .collect()
}

fn keys_into_syntax_loc<K: AstNode, V>(
    map: HashMap<K, V>,
    file_id: FileId,
) -> HashMap<loc::SyntaxLoc, V> {
    map.into_iter()
        .map(|(method_call, opt_entry)| (method_call.loc(file_id), opt_entry))
        .collect()
}
