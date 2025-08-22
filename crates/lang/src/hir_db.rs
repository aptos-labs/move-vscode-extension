// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::item_scope::NamedItemScope;
use crate::loc::{SyntaxLoc, SyntaxLocFileExt, SyntaxLocInput};
use crate::nameres::address::{Address, AddressInput};
use crate::nameres::node_ext::ModuleResolutionExt;
use crate::nameres::path_resolution;
use crate::nameres::scope::ScopeEntry;
use crate::nameres::use_speck_entries::{UseItem, use_items_for_stmt};
use crate::node_ext::ModuleLangExt;
use crate::node_ext::item::ModuleItemExt;
use crate::types::inference::InferenceCtx;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::inference::inference_result::InferenceResult;
use crate::types::ty::Ty;
use crate::{hir_db, item_scope, nameres};
use base_db::inputs::{FileIdInput, InternFileId};
use base_db::package_root::PackageId;
use base_db::{SourceDatabase, source_db};
use std::collections::HashSet;
use syntax::ast::UseStmtsOwner;
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
    stmts_owner: &InFile<impl ast::UseStmtsOwner>,
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
    let _p = tracing::debug_span!("inference_tracked").entered();

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
        let source_file_ids = source_file_ids_in_package(db, *dep_package_id);
        for source_file_id in source_file_ids {
            let modules = get_modules_in_file(db, *source_file_id, address.clone());
            if !modules.is_empty() {
                files_with_modules.push(*source_file_id);
            }
        }
    }
    files_with_modules
}

pub fn modules_for_package_id<'db>(
    db: &'db dyn SourceDatabase,
    package_id: PackageId,
) -> Vec<SyntaxLoc> {
    let source_file_ids = source_file_ids_in_package(db, package_id);
    let mut all_locs = vec![];
    for source_file_id in source_file_ids {
        let module_locs = get_all_modules_in_file(db, source_file_id.intern(db));
        all_locs.extend(module_locs);
    }
    all_locs
}

// #[salsa_macros::tracked(returns(ref))]
// pub fn all_package_file_ids(db: &dyn SourceDatabase, package_id: PackageId) -> Vec<FileId> {
//     let dep_package_ids = transitive_dep_package_ids(db, package_id);
//     let file_ids = vec![];
//     let file_sets = dep_package_ids
//         .iter()
//         .map(|it| db.package_root(*it).data(db).file_set.clone())
//         .collect();
//     // let file_sets = iter::once(package_id)
//     //     .chain(dep_package_idsids)
//     //     .map(|id| db.package_root(id).data(db).file_set.clone())
//     //     .collect::<Vec<_>>();
//
//     let mut source_file_ids = vec![];
//     for file_set in file_sets.clone() {
//         for source_file_id in file_set.iter() {
//             source_file_ids.push(source_file_id);
//         }
//     }
//     source_file_ids
// }

#[salsa_macros::tracked(returns(ref))]
pub fn source_file_ids_in_package(db: &dyn SourceDatabase, package_id: PackageId) -> Vec<FileId> {
    let file_set = &db.package_root(package_id).data(db).file_set;
    file_set.iter().collect()
}

/// returns packages dependencies, including package itself
#[salsa_macros::tracked(returns(ref))]
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

pub const APTOS_FRAMEWORK_ADDRESSES: [&str; 5] = [
    "std",
    "aptos_std",
    "aptos_framework",
    "aptos_token",
    "aptos_experimental",
];

pub fn named_addresses(db: &dyn SourceDatabase, package_id: Option<PackageId>) -> HashSet<String> {
    let mut all_addresses = HashSet::new();

    // add default addresses
    for std_address in APTOS_FRAMEWORK_ADDRESSES.map(|it| it.to_string()) {
        all_addresses.insert(std_address);
    }

    if let Some(package_id) = package_id {
        all_addresses.extend(named_addresses_tracked(db, package_id));
    }

    all_addresses
}

#[salsa_macros::tracked()]
pub fn named_addresses_tracked(db: &dyn SourceDatabase, package_id: PackageId) -> HashSet<String> {
    let mut all_addresses = HashSet::new();

    let all_package_ids = hir_db::transitive_dep_package_ids(db, package_id);
    for package_id in all_package_ids {
        if let Some(package_metadata) = source_db::metadata_for_package_id(db, *package_id) {
            all_addresses.extend(package_metadata.named_addresses);
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

pub fn item_scope(db: &dyn SourceDatabase, syntax_loc: SyntaxLoc) -> NamedItemScope {
    let file_item_scopes = item_scope::item_scopes(db, syntax_loc.file_id().intern(db));
    file_item_scopes
        .get(&syntax_loc.syntax_ptr())
        .cloned()
        .unwrap_or(NamedItemScope::Main)
}

pub fn use_items_from_self_and_siblings(
    db: &dyn SourceDatabase,
    use_stmts_owner: InFile<ast::AnyUseStmtsOwner>,
) -> Vec<UseItem> {
    use_items_from_self_and_siblings_tracked(db, SyntaxLocInput::new(db, use_stmts_owner.loc()))
}

fn use_items_from_self_and_siblings_tracked<'db>(
    db: &'db dyn SourceDatabase,
    use_stmts_owner_loc: SyntaxLocInput<'db>,
) -> Vec<UseItem> {
    let owner_with_siblings = use_stmts_owner_loc
        .to_ast::<ast::AnyUseStmtsOwner>(db)
        .map(|use_stmts_owner| use_stmts_owner_with_siblings(db, use_stmts_owner))
        .unwrap_or_default();
    owner_with_siblings
        .into_iter()
        .flat_map(|it| use_items(db, it))
        .collect()
}

pub fn use_stmts_owner_with_siblings(
    db: &dyn SourceDatabase,
    use_stmts_owner: InFile<ast::AnyUseStmtsOwner>,
) -> Vec<InFile<ast::AnyUseStmtsOwner>> {
    let mut with_siblings = vec![use_stmts_owner.clone()];
    if let Some(module) = use_stmts_owner.cast_into_ref::<ast::Module>() {
        with_siblings.extend(
            module
                .related_module_specs(db)
                .into_iter()
                .map(|it| it.map_into()),
        );
    }
    if let Some(module_spec) = use_stmts_owner.cast_into_ref::<ast::ModuleSpec>() {
        if let Some(module) = module_spec.module(db) {
            with_siblings.push(module.clone().map_into());
        }
    }
    with_siblings
}

pub fn use_items(
    db: &dyn SourceDatabase,
    use_stmts_owner: InFile<impl Into<ast::AnyUseStmtsOwner>>,
) -> Vec<UseItem> {
    use_items_tracked(
        db,
        SyntaxLocInput::new(db, use_stmts_owner.map(|it| it.into()).loc()),
    )
}

#[salsa_macros::tracked]
fn use_items_tracked<'db>(
    db: &'db dyn SourceDatabase,
    use_stmts_owner: SyntaxLocInput<'db>,
) -> Vec<UseItem> {
    use_stmts_owner
        .to_ast::<ast::AnyUseStmtsOwner>(db)
        .map(|use_stmts_owner| {
            let use_stmts = use_stmts_owner.flat_map(|it| it.use_stmts().collect());
            use_stmts
                .into_iter()
                .flat_map(|stmt| use_items_for_stmt(db, stmt).unwrap_or_default())
                .collect()
        })
        .unwrap_or_default()
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
            if module_address.equals_to(db, file_id, &address, false) {
                module_candidates.push(module);
            }
        }
    }
    module_candidates
    // let modules = source_file
    //     .all_modules()
    //     .filter(|m| m.address_equals_to(db, file_id, address.clone(), false))
    //     .collect::<Vec<_>>();
    // modules
}

#[salsa_macros::tracked]
pub(crate) fn get_all_modules_in_file(db: &dyn SourceDatabase, file_id: FileIdInput) -> Vec<SyntaxLoc> {
    let source_file = source_db::parse(db, file_id).tree();
    let modules = source_file.all_modules().collect::<Vec<_>>();
    let module_locs = modules
        .into_iter()
        .map(|it| it.in_file(file_id.data(db)).loc())
        .collect::<Vec<_>>();
    module_locs
}
