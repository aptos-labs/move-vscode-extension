use crate::nameres::lexical_declarations::process_nested_scopes_upwards;
use crate::nameres::name_resolution::process_module_path_resolve_variants;
use crate::nameres::path_kind::{path_kind, PathKind, QualifiedKind};
use crate::nameres::processors::{
    collect_entries, collect_entries_with_ref_name, filter_ns_processor, ProcessingStatus, Processor,
};
use crate::nameres::scope::ScopeEntry;
use crate::AsName;
use syntax::ast;

pub fn resolve_path_to_single_item(path: ast::Path) -> Option<ScopeEntry> {
    let entries = resolve_path(path);
    match entries.len() {
        0 => None,
        1 => entries.into_iter().next(),
        _ => None,
    }
}

pub fn resolve_path(path: ast::Path) -> Vec<ScopeEntry> {
    let ctx = PathResolutionContext {
        path: path.clone(),
        is_completion: false,
    };
    if let Some(path_kind) = path_kind(path.clone(), false) {
        if let Some(ref_name) = path.name_ref().map(|it| it.as_name().as_str().to_string()) {
            let entries = collect_entries_with_ref_name(ref_name, |resolver| {
                process_path_resolve_variants(ctx, path_kind, resolver);
            });
            return entries;
        }
    }
    vec![]
}

pub fn collect_paths_for_completion(path: ast::Path) -> Vec<ScopeEntry> {
    let ctx = PathResolutionContext {
        path: path.clone(),
        is_completion: true,
    };
    let mut entries = vec![];
    if let Some(path_kind) = path_kind(path.clone(), true) {
        entries.extend(collect_entries(|collector| {
            process_path_resolve_variants(ctx, path_kind, collector);
        }))
    }
    entries
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathResolutionContext {
    pub path: ast::Path,
    pub is_completion: bool,
}

pub fn process_path_resolve_variants(
    ctx: PathResolutionContext,
    path_kind: PathKind,
    processor: &impl Processor,
) -> ProcessingStatus {
    match path_kind {
        PathKind::NamedAddress(_) | PathKind::ValueAddress(_) => {
            // no path resolution for named / value addresses
            ProcessingStatus::Continue
        }
        PathKind::NamedAddressOrUnqualifiedPath { ns, .. } | PathKind::Unqualified { ns } => {
            // todo: resolve Self module
            // local
            let with_ns_filter = filter_ns_processor(ns, processor);
            process_nested_scopes_upwards(ctx, &with_ns_filter)
        }

        PathKind::Qualified {
            kind: QualifiedKind::Module { address },
            ns,
            ..
        } => process_module_path_resolve_variants(ctx, address, &filter_ns_processor(ns, processor)),

        PathKind::Qualified { qualifier, ns, .. } => {
            process_qualified_path_resolve_variants(ctx, qualifier, &filter_ns_processor(ns, processor))
        }
    }
}

fn process_qualified_path_resolve_variants(
    ctx: PathResolutionContext,
    qualifier: ast::Path,
    processor: &impl Processor,
) -> ProcessingStatus {
    let qualifier_item = resolve_path_to_single_item(qualifier);
    ProcessingStatus::Continue
}
