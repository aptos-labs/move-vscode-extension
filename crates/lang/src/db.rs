use crate::loc::{SyntaxLoc, SyntaxLocFileExt};
use crate::nameres::path_resolution;
use crate::nameres::scope::{ScopeEntry, VecExt};
use crate::types::inference::InferenceCtx;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::inference::inference_result::InferenceResult;
use crate::types::ty::Ty;
use base_db::{SourceDatabase, Upcast};
use std::sync::Arc;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxNodeExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};

#[ra_salsa::query_group(HirDatabaseStorage)]
pub trait HirDatabase: SourceDatabase + Upcast<dyn SourceDatabase> {
    fn resolve_path(&self, path_loc: SyntaxLoc) -> Option<ScopeEntry>;
    fn inference_for_ctx_owner(&self, ctx_owner_loc: SyntaxLoc, msl: bool) -> Arc<InferenceResult>;
}

pub(crate) fn resolve_path(db: &dyn HirDatabase, path_loc: SyntaxLoc) -> Option<ScopeEntry> {
    let path = path_loc.to_ast::<ast::Path>(db.upcast());
    match path {
        Some(path) => path_resolution::resolve_path(db, path, None).single_or_none(),
        None => {
            tracing::error!(
                ?path_loc,
                "resolve_path() function should only receive loc of Path, this is a bug"
            );
            None
        }
    }
}

#[tracing::instrument(level = "debug", skip(db))]
fn inference_for_ctx_owner(
    db: &dyn HirDatabase,
    ctx_owner_loc: SyntaxLoc,
    msl: bool,
) -> Arc<InferenceResult> {
    let InFile { file_id, value: ctx_owner } = ctx_owner_loc
        .to_ast::<ast::InferenceCtxOwner>(db.upcast())
        .unwrap();
    let mut ctx = InferenceCtx::new(db, file_id, msl);

    let return_ty = if let Some(any_fun) = ctx_owner.syntax().clone().cast::<ast::AnyFun>() {
        let ret_ty = ctx
            .ty_lowering()
            .lower_any_function(any_fun.in_file(file_id).map_into())
            .ret_type();
        ret_ty
    } else {
        Ty::Unknown
    };

    let mut type_walker = TypeAstWalker::new(&mut ctx, return_ty);
    type_walker.walk(ctx_owner);

    Arc::new(InferenceResult::from_ctx(ctx))
}

pub trait NodeInferenceExt {
    fn inference(&self, db: &dyn HirDatabase, msl: bool) -> Option<Arc<InferenceResult>>;
}

impl<T: AstNode> NodeInferenceExt for InFile<T> {
    fn inference(&self, db: &dyn HirDatabase, msl: bool) -> Option<Arc<InferenceResult>> {
        let (file_id, node) = self.unpack_ref();
        let inference_owner = node.syntax().ancestor_or_self::<ast::InferenceCtxOwner>()?;
        let inference = db.inference_for_ctx_owner(inference_owner.in_file(file_id).loc(), msl);
        Some(inference)
    }
}
