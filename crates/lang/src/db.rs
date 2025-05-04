use crate::loc::{SyntaxLoc, SyntaxLocFileExt};
use crate::nameres::address::Address;
use crate::nameres::path_resolution;
use crate::nameres::scope::{NamedItemsExt, ScopeEntry, VecExt};
use crate::node_ext::ModuleLangExt;
use crate::types::inference::InferenceCtx;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::inference::inference_result::InferenceResult;
use crate::types::ty::Ty;
use base_db::{ParseDatabase, SourceDatabase};
use base_db::package_root::PackageRootId;
use std::fs::File;
use std::sync::Arc;
use syntax::ast::Module;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxNodeExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt, InFileVecExt};
use syntax::{AstNode, ast};
use vfs::FileId;
use base_db::inputs::{FileIdSet, InternFileId};

#[query_group_macro::query_group]
pub trait HirDatabase: ParseDatabase {
    fn resolve_path(&self, path_loc: SyntaxLoc) -> Vec<ScopeEntry>;

    fn inference_for_ctx_owner(&self, ctx_owner_loc: SyntaxLoc, msl: bool) -> Arc<InferenceResult>;

    fn file_ids_by_module_address(&self, package_root_id: PackageRootId, address: Address) -> FileIdSet;
}

pub(crate) fn resolve_path(db: &dyn HirDatabase, path_loc: SyntaxLoc) -> Vec<ScopeEntry> {
    let path = path_loc.to_ast::<ast::Path>(db);
    match path {
        Some(path) => path_resolution::resolve_path(db, path, None),
        None => {
            tracing::error!(
                ?path_loc,
                "resolve_path() function should only receive loc of Path, this is a bug"
            );
            vec![]
        }
    }
}

#[tracing::instrument(level = "debug", skip(db))]
fn inference_for_ctx_owner(
    db: &dyn HirDatabase,
    ctx_owner_loc: SyntaxLoc,
    msl: bool,
) -> Arc<InferenceResult> {
    let InFile { file_id, value: ctx_owner } =
        ctx_owner_loc.to_ast::<ast::InferenceCtxOwner>(db).unwrap();
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

fn file_ids_by_module_address(
    db: &dyn HirDatabase,
    package_root_id: PackageRootId,
    address: Address,
) -> FileIdSet {
    let source_file_ids = db.source_file_ids(package_root_id).data(db);

    let mut file_ids = vec![];
    for source_file_id in source_file_ids {
        let modules = get_modules_in_file(db, source_file_id, address.clone());
        if !modules.is_empty() {
            file_ids.push(source_file_id);
        }
    }
    FileIdSet::new(db, file_ids)
}

pub(crate) fn get_modules_in_file(
    db: &dyn ParseDatabase,
    file_id: FileId,
    address: Address,
) -> Vec<Module> {
    let source_file = db.parse(file_id.intern(db)).tree();
    let modules = source_file
        .all_modules()
        .filter(|m| m.address_equals_to(address.clone(), false))
        .collect::<Vec<_>>();
    modules
}
