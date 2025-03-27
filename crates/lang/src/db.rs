use crate::InFile;
use crate::files::InFileExt;
use crate::loc::{SyntaxLoc, SyntaxLocFileExt};
use crate::nameres::path_resolution;
use crate::nameres::scope::{ScopeEntry, VecExt};
use crate::types::inference::InferenceCtx;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::inference::inference_result::InferenceResult;
use base_db::{SourceRootDatabase, Upcast};
use syntax::ast;
use triomphe::Arc;

#[ra_salsa::query_group(HirDatabaseStorage)]
pub trait HirDatabase: SourceRootDatabase + Upcast<dyn SourceRootDatabase> {
    fn resolve_path(&self, path_loc: SyntaxLoc) -> Option<ScopeEntry>;
    fn inference_for_ctx_owner(&self, ctx_owner_loc: SyntaxLoc) -> Arc<InferenceResult>;
}

fn resolve_path(db: &dyn HirDatabase, ref_loc: SyntaxLoc) -> Option<ScopeEntry> {
    let path = ref_loc.to_ast::<ast::Path>(db.upcast())?;
    path_resolution::resolve_path(db, path).single_or_none()
}

fn inference_for_ctx_owner(db: &dyn HirDatabase, ctx_owner_loc: SyntaxLoc) -> Arc<InferenceResult> {
    let InFile {
        file_id,
        value: ctx_owner,
    } = ctx_owner_loc
        .to_ast::<ast::InferenceCtxOwner>(db.upcast())
        .unwrap();
    let mut ctx = InferenceCtx::new(db, file_id);

    let mut type_walker = TypeAstWalker::new(&mut ctx);
    type_walker.walk(ctx_owner);

    Arc::new(InferenceResult::from_ctx(ctx))
}

impl InFile<ast::InferenceCtxOwner> {
    pub fn inference(&self, db: &dyn HirDatabase) -> Arc<InferenceResult> {
        let ctx_owner_loc = self.loc();
        db.inference_for_ctx_owner(ctx_owner_loc)
    }
}
