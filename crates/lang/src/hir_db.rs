// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

mod use_items_ext;

use crate::item_scope::ItemScope;
use crate::loc::{SyntaxLoc, SyntaxLocFileExt, SyntaxLocInput};
use crate::nameres::address::{Address, AddressInput};
use crate::nameres::is_visible::ScopeEntryWithVis;
use crate::nameres::node_ext::ModuleResolutionExt;
use crate::nameres::path_resolution;
use crate::nameres::scope::{ScopeEntry, ScopeEntryExt};
use crate::node_ext::ModuleLangExt;
use crate::types::inference::InferenceCtx;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::inference::inference_result::InferenceResult;
use crate::types::ty::Ty;
use crate::types::ty_db;
use crate::{hir_db, item_scope, nameres};
use base_db::inputs::{FileIdInput, InternFileId};
use base_db::package_root::PackageId;
use base_db::{SourceDatabase, source_db};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use syntax::ast;
use syntax::ast::UseStmtsOwner;
use syntax::files::{InFile, InFileExt};
use vfs::FileId;

pub use use_items_ext::*;

pub(crate) fn resolve_path_multi(
    db: &dyn SourceDatabase,
    path: InFile<ast::Path>,
) -> Vec<ScopeEntryWithVis> {
    resolve_path_multi_tracked(db, SyntaxLocInput::new(db, path.loc()))
}

#[salsa_macros::tracked]
fn resolve_path_multi_tracked<'db>(
    db: &'db dyn SourceDatabase,
    path_loc: SyntaxLocInput<'db>,
) -> Vec<ScopeEntryWithVis> {
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
    stmts_owner: &InFile<impl UseStmtsOwner>,
) -> Vec<ScopeEntry> {
    use_speck_entries_tracked(db, SyntaxLocInput::new(db, stmts_owner.loc()))
}

#[salsa_macros::tracked]
fn use_speck_entries_tracked<'db>(
    db: &'db dyn SourceDatabase,
    stmts_owner_loc: SyntaxLocInput<'db>,
) -> Vec<ScopeEntry> {
    let use_stmts_owner = stmts_owner_loc.to_ast::<ast::AnyUseStmtsOwner>(db).unwrap();
    let entries = nameres::use_speck_entries::use_speck_entries(db, use_stmts_owner);
    entries
}

pub(crate) fn inference(db: &dyn SourceDatabase, owner_loc: SyntaxLoc, msl: bool) -> &InferenceResult {
    let _p = tracing::debug_span!("inference").entered();
    inference_tracked(db, SyntaxLocInput::new(db, owner_loc), msl)
}

#[salsa_macros::tracked(returns(ref))]
fn inference_tracked<'db>(
    db: &'db dyn SourceDatabase,
    ctx_owner_loc: SyntaxLocInput<'db>,
    msl: bool,
) -> InferenceResult {
    let ctx_owner = ctx_owner_loc.to_ast::<ast::InferenceCtxOwner>(db).unwrap();

    let return_ty = match ctx_owner.syntax().syntax_cast::<ast::AnyFun>() {
        Some(fun) => ty_db::lower_function(db, fun, msl).ret_type_ty(),
        None => Ty::Unknown,
    };

    let (file_id, ctx_owner) = ctx_owner.unpack();
    let mut ctx = InferenceCtx::new(db, file_id, msl);
    let mut type_walker = TypeAstWalker::new(&mut ctx, return_ty);
    type_walker.walk(ctx_owner);

    InferenceResult::from_ctx(ctx)
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

    let mut files_with_modules = vec![];
    let dep_package_ids = transitive_dep_package_ids(db, package_id);
    for dep_package_id in dep_package_ids {
        let source_file_ids = source_file_ids_in_package(db, dep_package_id);
        for source_file_id in source_file_ids {
            let modules = get_modules_in_file(db, *source_file_id, address.clone());
            if !modules.is_empty() {
                files_with_modules.push(*source_file_id);
            }
        }
    }
    files_with_modules
}

pub fn get_all_modules_for_package_id(
    db: &dyn SourceDatabase,
    package_id: PackageId,
) -> impl Iterator<Item = InFile<ast::Module>> {
    get_all_modules_for_package_id_tracked(db, package_id)
        .iter()
        .filter_map(|it| it.to_ast::<ast::Module>(db))
}

#[salsa_macros::tracked(returns(ref))]
fn get_all_modules_for_package_id_tracked(
    db: &dyn SourceDatabase,
    package_id: PackageId,
) -> Vec<SyntaxLoc> {
    let source_file_ids = source_file_ids_in_package(db, package_id);
    // average is one module per file
    let mut all_module_locs = Vec::with_capacity(source_file_ids.len());
    for source_file_id in source_file_ids {
        let module_locs = get_all_modules_in_file(db, source_file_id.intern(db));
        all_module_locs.extend(module_locs);
    }
    all_module_locs
}

pub fn import_candidates(db: &dyn SourceDatabase, file_id: FileId) -> &Vec<ScopeEntry> {
    import_candidates_tracked(db, file_id.intern(db))
}

#[salsa_macros::tracked(returns(ref))]
pub fn import_candidates_tracked(db: &dyn SourceDatabase, file_id: FileIdInput) -> Vec<ScopeEntry> {
    let _p = tracing::debug_span!("import_candidates_tracked").entered();

    let current_package_id = db.file_package_id(file_id.data(db));
    let all_package_ids = hir_db::transitive_dep_package_ids(db, current_package_id);
    let mut all_candidates = vec![];
    for package_id in all_package_ids {
        for module in get_all_modules_for_package_id(db, package_id) {
            all_candidates.extend(module.clone().to_entry());
            all_candidates.extend(module.importable_entries());
        }
    }
    all_candidates
}

#[salsa_macros::tracked(returns(ref))]
pub fn source_file_ids_in_package(db: &dyn SourceDatabase, package_id: PackageId) -> Vec<FileId> {
    let file_set = &db.package_root(package_id).data(db).file_set;
    file_set.iter().collect()
}

/// returns packages dependencies, including package itself
#[salsa_macros::tracked]
pub fn transitive_dep_package_ids(db: &dyn SourceDatabase, package_id: PackageId) -> Vec<PackageId> {
    let metadata = source_db::metadata_for_package_id(db, package_id);
    match metadata {
        None => vec![package_id],
        Some(metadata) => {
            let mut entries = vec![package_id];
            let dep_manifest_ids = metadata.dep_manifest_ids;
            entries.extend(
                dep_manifest_ids
                    .iter()
                    .map(|file_id| db.file_package_id(*file_id)),
            );
            entries
        }
    }
}

pub fn missing_dependencies(db: &dyn SourceDatabase, package_id: PackageId) -> Vec<String> {
    let all_package_ids = self::transitive_dep_package_ids(db, package_id);
    let mut missing_dependencies = HashSet::new();
    for package_id in all_package_ids {
        if let Some(package_metadata) = source_db::metadata_for_package_id(db, package_id) {
            missing_dependencies.extend(package_metadata.missing_dependencies);
        }
    }
    missing_dependencies.into_iter().sorted().collect()
}

/// returns reverse package dependencies, including package itself
#[salsa_macros::tracked]
pub fn reverse_transitive_dep_package_ids(db: &dyn SourceDatabase, of: PackageId) -> Vec<PackageId> {
    // todo: can be sped up, for now just do dumb version
    let mut rev_deps = vec![];
    rev_deps.push(of);
    for dep_id in db.all_package_ids().data(db) {
        let transitive_deps = transitive_dep_package_ids(db, dep_id);
        if transitive_deps.contains(&of) {
            rev_deps.push(dep_id);
        }
    }
    rev_deps
}

pub const APTOS_FRAMEWORK_ADDRESSES: [&str; 4] = ["std", "aptos_std", "aptos_framework", "aptos_token"];

pub fn named_addresses(db: &dyn SourceDatabase) -> HashMap<String, String> {
    // let mut all_addresses = HashSet::new();

    // add default addresses
    named_addresses_tracked(db)
    // all_addresses.extend(named_addresses_tracked(db));

    // for std_address in APTOS_FRAMEWORK_ADDRESSES.map(|it| it.to_string()) {
    //     if !all_addresses.contains(std_address) {
    //
    //     }
    //     all_addresses.insert((std_address, "0x1".to_string()));
    // }

    // all_addresses
}

#[salsa_macros::tracked]
pub fn named_addresses_tracked(db: &dyn SourceDatabase) -> HashMap<String, String> {
    let mut all_addresses = HashMap::new();

    let all_package_ids = db.all_package_ids();
    for package_id in all_package_ids.data(db) {
        if let Some(package_metadata) = source_db::metadata_for_package_id(db, package_id) {
            for (address_name, address_val) in package_metadata.named_addresses {
                all_addresses.insert(address_name, address_val);
            }
        }
    }

    all_addresses
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

pub fn item_scope(db: &dyn SourceDatabase, syntax_loc: SyntaxLoc) -> ItemScope {
    let file_item_scopes = item_scope::item_scopes(db, syntax_loc.file_id().intern(db));
    file_item_scopes
        .get(&syntax_loc.syntax_ptr())
        .cloned()
        .unwrap_or(ItemScope::Main)
}

pub(crate) fn get_modules_in_file(
    db: &dyn SourceDatabase,
    file_id: FileId,
    address: Address,
) -> Vec<ast::Module> {
    let source_file = source_db::parse(db, file_id.intern(db)).tree();
    let mut module_candidates = vec![];
    for module in source_file.all_modules() {
        if let Some(module_address) = module.address() {
            if module_address.equals_to(db, &address, false) {
                module_candidates.push(module);
            }
        }
    }
    module_candidates
}

pub(crate) fn get_all_modules_in_file(db: &dyn SourceDatabase, file_id: FileIdInput) -> Vec<SyntaxLoc> {
    let source_file = source_db::parse(db, file_id).tree();
    let file_id = file_id.data(db);
    let module_locs = source_file
        .all_modules()
        .into_iter()
        .map(|it| it.in_file(file_id).loc())
        .collect::<Vec<_>>();
    module_locs
}
