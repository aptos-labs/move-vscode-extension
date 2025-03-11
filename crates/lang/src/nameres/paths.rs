use crate::db::HirDatabase;
use crate::nameres::lexical_declarations::process_nested_scopes_upwards;
use crate::nameres::name_resolution::get_entries_from_walking_scopes;
use crate::nameres::namespaces::{Ns, MODULES};
use crate::nameres::path_kind::{path_kind, PathKind, QualifiedKind};
use crate::nameres::paths;
use crate::nameres::processors::{
    collect_entries, collect_entries_with_ref_name, filter_ns_processor, ProcessingStatus, Processor,
};
use crate::nameres::scope::ScopeEntry;
use crate::{AsName, Name};
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxNodeExt;
use syntax::{ast, AstNode};

pub fn get_path_resolve_variants(
    db: &dyn HirDatabase,
    ctx: ResolutionContext,
    path_kind: PathKind,
) -> Vec<ScopeEntry> {
    let mut entries = vec![];
    match path_kind {
        PathKind::NamedAddress(_) | PathKind::ValueAddress(_) => {
            // no path resolution for named / value addresses
        }
        PathKind::NamedAddressOrUnqualifiedPath { ns, .. } | PathKind::Unqualified { ns } => {
            if ns.contains(Ns::MODULE) {
                if let Some(module) = ctx.containing_module() {
                    entries.push(ScopeEntry {
                        name: Name::new("Self"),
                        named_node: module.syntax().to_owned(),
                        ns: MODULES,
                    })
                }
            }
            entries.extend(get_entries_from_walking_scopes(db, ctx.path, ns))
        }

        PathKind::Qualified {
            kind: QualifiedKind::Module { address },
            ns,
            ..
        } => {
            // process_module_path_resolve_variants(ctx, address, &filter_ns_processor(ns, processor))
            // ProcessingStatus::Continue
        }

        PathKind::Qualified { qualifier, ns, .. } => {
            // process_qualified_path_resolve_variants(ctx, qualifier, &filter_ns_processor(ns, processor))
        }
    }
    entries
}

pub fn resolve_path_to_single_item(db: &dyn HirDatabase, path: ast::Path) -> Option<ScopeEntry> {
    // let path_loc = SyntaxLoc::from_syntax_node(path);
    // let entries = db.resolve_ast_path(path_loc);
    // let path_loc = SyntaxLoc::from_syntax_node(path);
    let entries = paths::resolve_path(path);
    match entries.len() {
        0 => None,
        1 => entries.into_iter().next(),
        _ => None,
    }
}

pub fn resolve_path(path: ast::Path) -> Vec<ScopeEntry> {
    let ctx = ResolutionContext {
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
    let ctx = ResolutionContext {
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
pub struct ResolutionContext {
    pub path: ast::Path,
    pub is_completion: bool,
}

impl ResolutionContext {
    pub fn containing_module(&self) -> Option<ast::Module> {
        self.path.syntax().containing_module()
    }
}

pub fn process_path_resolve_variants(
    ctx: ResolutionContext,
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
        } => {
            // process_module_path_resolve_variants(ctx, address, &filter_ns_processor(ns, processor))
            ProcessingStatus::Continue
        }

        PathKind::Qualified { qualifier, ns, .. } => {
            process_qualified_path_resolve_variants(ctx, qualifier, &filter_ns_processor(ns, processor))
        }
    }
}

fn process_qualified_path_resolve_variants(
    ctx: ResolutionContext,
    qualifier: ast::Path,
    processor: &impl Processor,
) -> ProcessingStatus {
    // let qualifier_item = resolve_path_to_single_item(qualifier);
    ProcessingStatus::Continue
}
