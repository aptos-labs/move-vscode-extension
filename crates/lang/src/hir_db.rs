use crate::item_scope::NamedItemScope;
use crate::loc::{SyntaxLoc, SyntaxLocFileExt, SyntaxLocInput};
use crate::nameres;
use crate::nameres::address::{Address, AddressInput};
use crate::nameres::node_ext::ModuleResolutionExt;
use crate::nameres::path_resolution;
use crate::nameres::scope::ScopeEntry;
use crate::node_ext::ModuleLangExt;
use crate::types::inference::InferenceCtx;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::inference::inference_result::InferenceResult;
use crate::types::ty::Ty;
use base_db::inputs::InternFileId;
use base_db::package_root::PackageId;
use base_db::{SourceDatabase, source_db};
use std::iter;
use std::sync::Arc;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};
use vfs::FileId;

pub(crate) fn resolve_path_multi(db: &dyn SourceDatabase, path: InFile<ast::Path>) -> Vec<ScopeEntry> {
    resolve_path_multi_tracked(db, SyntaxLocInput::new(db, path.loc()))
}

#[salsa_macros::tracked]
fn resolve_path_multi_tracked<'db>(
    db: &'db dyn SourceDatabase,
    path_loc: SyntaxLocInput<'db>,
) -> Vec<ScopeEntry> {
    let path = path_loc.to_ast::<ast::Path>(db);
    match path {
        Some(path) => path_resolution::resolve_path(db, path, None),
        None => {
            tracing::error!(
                path_loc = ?path_loc.syntax_loc(db),
                "resolve_path() function should only receive loc of Path, this is a bug"
            );
            vec![]
        }
    }
}

pub(crate) fn use_speck_entries(
    db: &dyn SourceDatabase,
    stmts_owner: InFile<ast::AnyHasUseStmts>,
) -> Vec<ScopeEntry> {
    use_speck_entries_tracked(db, SyntaxLocInput::new(db, stmts_owner.loc()))
}

#[salsa_macros::tracked]
fn use_speck_entries_tracked<'db>(
    db: &'db dyn SourceDatabase,
    stmts_owner_loc: SyntaxLocInput<'db>,
) -> Vec<ScopeEntry> {
    let use_stmts_owner = stmts_owner_loc.to_ast::<ast::AnyHasUseStmts>(db).unwrap();
    let entries = nameres::use_speck_entries::use_speck_entries(db, &use_stmts_owner);
    entries
}

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn inference(
    db: &dyn SourceDatabase,
    inference_owner: InFile<ast::InferenceCtxOwner>,
    msl: bool,
) -> Arc<InferenceResult> {
    inference_tracked(db, SyntaxLocInput::new(db, inference_owner.loc()), msl)
}

#[tracing::instrument(level = "debug", skip(db))]
#[salsa_macros::tracked]
fn inference_tracked<'db>(
    db: &'db dyn SourceDatabase,
    ctx_owner_loc: SyntaxLocInput<'db>,
    msl: bool,
) -> Arc<InferenceResult> {
    let (file_id, ctx_owner) = ctx_owner_loc
        .to_ast::<ast::InferenceCtxOwner>(db)
        .unwrap()
        .unpack();
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
    fn inference(&self, db: &dyn SourceDatabase, msl: bool) -> Option<Arc<InferenceResult>>;
}

impl<T: AstNode> NodeInferenceExt for InFile<T> {
    fn inference(&self, db: &dyn SourceDatabase, msl: bool) -> Option<Arc<InferenceResult>> {
        let ctx_owner = self.and_then_ref(|it| it.syntax().inference_ctx_owner())?;
        let inference = inference(db, ctx_owner, msl);
        Some(inference)
    }
}

pub(crate) fn file_ids_by_module_address(
    db: &dyn SourceDatabase,
    package_id: PackageId,
    address: Address,
) -> Vec<FileId> {
    file_ids_by_module_address_tracked(db, package_id, AddressInput::new(db, address))
}

#[salsa_macros::tracked]
fn file_ids_by_module_address_tracked<'db>(
    db: &'db dyn SourceDatabase,
    package_id: PackageId,
    address: AddressInput<'db>,
) -> Vec<FileId> {
    let address = address.data(db);
    let source_file_ids = all_package_file_ids(db, package_id);
    let mut file_ids = vec![];
    for source_file_id in source_file_ids {
        let modules = get_modules_in_file(db, source_file_id, address.clone());
        if !modules.is_empty() {
            file_ids.push(source_file_id);
        }
    }
    file_ids
}

#[salsa_macros::tracked]
fn all_package_file_ids(db: &dyn SourceDatabase, package_id: PackageId) -> Vec<FileId> {
    let dep_ids = dep_package_ids(db, package_id);
    let file_sets = iter::once(package_id)
        .chain(dep_ids)
        .map(|id| db.package_root(id).data(db).file_set.clone())
        .collect::<Vec<_>>();

    let mut source_file_ids = vec![];
    for file_set in file_sets.clone() {
        for source_file_id in file_set.iter() {
            source_file_ids.push(source_file_id);
        }
    }
    source_file_ids
}

#[salsa_macros::tracked]
fn dep_package_ids(db: &dyn SourceDatabase, package_id: PackageId) -> Vec<PackageId> {
    let Some(package_manifest_id) = db.package_root(package_id).data(db).manifest_file_id else {
        return vec![];
    };
    let dep_manifest_ids = db.dep_package_ids(package_manifest_id).dep_manifests(db);
    dep_manifest_ids
        .iter()
        .map(|it| db.file_package_id(*it))
        .collect()
}

pub(crate) fn module_importable_entries(
    db: &dyn SourceDatabase,
    module_loc: SyntaxLoc,
) -> Vec<ScopeEntry> {
    #[salsa_macros::tracked]
    fn module_importable_entries_tracked<'db>(
        db: &'db dyn SourceDatabase,
        module_loc: SyntaxLocInput<'db>,
    ) -> Vec<ScopeEntry> {
        module_loc
            .to_ast::<ast::Module>(db)
            .map(|it| it.importable_entries())
            .unwrap_or_default()
    }
    module_importable_entries_tracked(db, SyntaxLocInput::new(db, module_loc))
}

pub(crate) fn module_importable_entries_from_related(
    db: &dyn SourceDatabase,
    module_loc: SyntaxLoc,
) -> Vec<ScopeEntry> {
    #[salsa_macros::tracked]
    fn module_importable_entries_from_related_tracked<'db>(
        db: &'db dyn SourceDatabase,
        module_loc: SyntaxLocInput<'db>,
    ) -> Vec<ScopeEntry> {
        module_loc
            .to_ast::<ast::Module>(db)
            .map(|it| it.importable_entries_from_related(db))
            .unwrap_or_default()
    }
    module_importable_entries_from_related_tracked(db, SyntaxLocInput::new(db, module_loc))
}

pub fn item_scope(db: &dyn SourceDatabase, syntax_loc: SyntaxLoc) -> NamedItemScope {
    #[salsa_macros::tracked]
    pub fn item_scope_tracked<'db>(
        db: &'db dyn SourceDatabase,
        loc: SyntaxLocInput<'db>,
    ) -> NamedItemScope {
        loc.syntax_loc(db).item_scope(db).unwrap_or(NamedItemScope::Main)
    }
    item_scope_tracked(db, SyntaxLocInput::new(db, syntax_loc))
}

pub(crate) fn get_modules_in_file(
    db: &dyn SourceDatabase,
    file_id: FileId,
    address: Address,
) -> Vec<ast::Module> {
    let source_file = source_db::parse(db, file_id.intern(db)).tree();
    let modules = source_file
        .all_modules()
        .filter(|m| m.address_equals_to(address.clone(), false))
        .collect::<Vec<_>>();
    modules
}
