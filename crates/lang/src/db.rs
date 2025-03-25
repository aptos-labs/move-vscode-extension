use crate::InFile;
use crate::loc::{SyntaxLoc, SyntaxLocFileExt};
use crate::types::inference::InferenceCtx;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::inference::inference_result::InferenceResult;
use base_db::{SourceRootDatabase, Upcast};
use syntax::ast;
use triomphe::Arc;

#[ra_salsa::query_group(HirDatabaseStorage)]
pub trait HirDatabase: SourceRootDatabase + Upcast<dyn SourceRootDatabase> {
    fn inference_for_ctx_owner(&self, ctx_owner_loc: SyntaxLoc) -> Arc<InferenceResult>;

    // #[ra_salsa::transparent]
    // fn inference(&self, ctx_owner: InFile<ast::InferenceCtxOwner>) -> Arc<InferenceResult>;
}

fn inference_for_ctx_owner(db: &dyn HirDatabase, ctx_owner_loc: SyntaxLoc) -> Arc<InferenceResult> {
    let InFile {
        file_id,
        value: ctx_owner,
    } = ctx_owner_loc
        .cast_into::<ast::InferenceCtxOwner>(db.upcast())
        .unwrap();
    let mut ctx = InferenceCtx::new(db, file_id);

    TypeAstWalker::new(&mut ctx).walk(ctx_owner);

    Arc::new(InferenceResult::from_ctx(ctx))
}

impl InFile<ast::InferenceCtxOwner> {
    pub fn inference(&self, db: &dyn HirDatabase) -> Arc<InferenceResult> {
        let ctx_owner_loc = self.loc();
        db.inference_for_ctx_owner(ctx_owner_loc)
    }
}

// fn inference(db: &dyn HirDatabase, ctx_owner: InFile<ast::InferenceCtxOwner>) -> InferenceResult {
//     let ctx_owner_loc = ctx_owner.loc();
//     db.inference_for_ctx_owner(ctx_owner_loc)
// }
