use crate::loc::{SyntaxLoc, SyntaxLocFileExt};
use crate::nameres::path_resolution;
use crate::nameres::scope::{ScopeEntry, VecExt};
use crate::types::inference::InferenceCtx;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::inference::inference_result::InferenceResult;
use crate::types::ty::Ty;
use base_db::{PackageRootDatabase, Upcast};
use parser::SyntaxKind;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};
use triomphe::Arc;

#[ra_salsa::query_group(HirDatabaseStorage)]
pub trait HirDatabase: PackageRootDatabase + Upcast<dyn PackageRootDatabase> {
    fn resolve_path(&self, path_loc: SyntaxLoc) -> Option<ScopeEntry>;
    fn inference_for_ctx_owner(&self, ctx_owner_loc: SyntaxLoc) -> Arc<InferenceResult>;
}

fn resolve_path(db: &dyn HirDatabase, ref_loc: SyntaxLoc) -> Option<ScopeEntry> {
    let path = ref_loc.to_ast::<ast::Path>(db.upcast())?;
    path_resolution::resolve_path(db, path, None).single_or_none()
}

#[tracing::instrument(level = "debug", skip(db))]
fn inference_for_ctx_owner(db: &dyn HirDatabase, ctx_owner_loc: SyntaxLoc) -> Arc<InferenceResult> {
    let InFile {
        file_id,
        value: ctx_owner,
    } = ctx_owner_loc
        .to_ast::<ast::InferenceCtxOwner>(db.upcast())
        .unwrap();
    let mut ctx = InferenceCtx::new(db, file_id);

    let return_ty = match ctx_owner.syntax().kind() {
        SyntaxKind::FUN => {
            let fun = ctx_owner.clone().fun().unwrap();
            let ret_ty = ctx.ty_lowering().lower_function(fun.in_file(file_id)).ret_type();
            ret_ty
        }
        _ => Ty::Unknown,
    };

    let mut type_walker = TypeAstWalker::new(&mut ctx, return_ty);
    type_walker.walk(ctx_owner);

    Arc::new(InferenceResult::from_ctx(ctx))
}

pub trait NodeInferenceExt {
    fn inference(&self, db: &dyn HirDatabase) -> Option<Arc<InferenceResult>>;
}

impl<T: AstNode> NodeInferenceExt for InFile<T> {
    fn inference(&self, db: &dyn HirDatabase) -> Option<Arc<InferenceResult>> {
        let (file_id, node) = self.unpack_ref();
        let inference_owner = node.syntax().ancestor_or_self::<ast::InferenceCtxOwner>()?;
        let inference = db.inference_for_ctx_owner(inference_owner.in_file(file_id).loc());
        Some(inference)
    }
}
