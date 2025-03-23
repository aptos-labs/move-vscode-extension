use crate::files::{InFileExt, InFileInto};
use crate::loc::{SyntaxLoc, SyntaxLocFileExt};
use crate::nameres::path_resolution;
use crate::nameres::scope::{ScopeEntry, ScopeEntryListExt};
use crate::node_ext::struct_field_name::StructFieldNameExt;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::inference::inference_result::InferenceResult;
use crate::types::inference::InferenceCtx;
use crate::InFile;
use base_db::{SourceRootDatabase, Upcast};
use stdx::itertools::Itertools;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::ast::{NamedElement, ReferenceElement};
use syntax::{ast, AstNode};

#[ra_salsa::query_group(HirDatabaseStorage)]
pub trait HirDatabase: SourceRootDatabase + Upcast<dyn SourceRootDatabase> {
    fn inference_for_ctx_owner(&self, ctx_owner_loc: SyntaxLoc) -> InferenceResult;

    #[ra_salsa::transparent]
    fn inference(&self, ctx_owner: InFile<ast::InferenceCtxOwner>) -> InferenceResult;
}

fn inference_for_ctx_owner(db: &dyn HirDatabase, ctx_owner_loc: SyntaxLoc) -> InferenceResult {
    let InFile {
        file_id,
        value: ctx_owner,
    } = ctx_owner_loc
        .cast_into::<ast::InferenceCtxOwner>(db.upcast())
        .unwrap();
    let mut ctx = InferenceCtx::new(db, file_id);

    TypeAstWalker::new(&mut ctx).walk(ctx_owner);

    InferenceResult::from_ctx(ctx)
}

fn inference(db: &dyn HirDatabase, ctx_owner: InFile<ast::InferenceCtxOwner>) -> InferenceResult {
    let ctx_owner_loc = ctx_owner.loc();
    db.inference_for_ctx_owner(ctx_owner_loc)
}
