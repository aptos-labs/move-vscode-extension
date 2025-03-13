use crate::db::HirDatabase;
use crate::files::OptionInFileExt;
use crate::loc::SyntaxLocExt;
use crate::nameres::lexical_declarations::process_nested_scopes_upwards;
use crate::nameres::name_resolution::{
    get_entries_from_walking_scopes, get_modules_as_entries, get_qualified_path_entries,
};
use crate::nameres::namespaces::{Ns, MODULES};
use crate::nameres::path_kind::{path_kind, PathKind, QualifiedKind};
use crate::nameres::processors::{filter_ns_processor, ProcessingStatus, Processor};
use crate::nameres::scope::{ScopeEntry, ScopeEntryListExt};
use crate::node_ext::PathLangExt;
use crate::{loc, InFile, Name};
use std::ops::Deref;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxNodeExt;
use syntax::{ast, AstNode};

pub fn get_path_resolve_variants(
    db: &dyn HirDatabase,
    ctx: ResolutionContext,
    path_kind: PathKind,
) -> Vec<ScopeEntry> {
    let mut entries = vec![];
    match path_kind {
        PathKind::Unknown => {}
        PathKind::NamedAddress(_) | PathKind::ValueAddress(_) => {
            // no path resolution for named / value addresses
        }
        PathKind::NamedAddressOrUnqualifiedPath { ns, .. } | PathKind::Unqualified { ns } => {
            if ns.contains(Ns::MODULE) {
                if let Some(module) = ctx.containing_module().opt_in_file(ctx.path.file_id) {
                    entries.push(ScopeEntry {
                        name: Name::new("Self"),
                        named_node_loc: module.loc(),
                        ns: MODULES,
                    })
                }
            }
            entries.extend(get_entries_from_walking_scopes(db, ctx, ns))
        }

        PathKind::Qualified {
            kind: QualifiedKind::Module { address },
            ..
        } => {
            entries.extend(get_modules_as_entries(db, ctx, address))
        }

        PathKind::Qualified { qualifier, .. } => {
            entries.extend(get_qualified_path_entries(db, ctx, qualifier))
        }
    }
    entries
}

pub fn resolve_single(db: &dyn HirDatabase, path: InFile<ast::Path>) -> Option<ScopeEntry> {
    let loc = loc::SyntaxLoc::from_ast_node(path);
    let entries = db.resolve_ast_path(loc);
    tracing::debug!(?entries);
    match entries.len() {
        1 => entries.into_iter().next(),
        _ => None,
    }
}

#[tracing::instrument(
    level = "debug",
    skip(db, path),
    fields(path = ?path.syntax_text()))]
pub fn resolve(db: &dyn HirDatabase, path: InFile<ast::Path>) -> Vec<ScopeEntry> {
    let Some(path_name) = path.value.name_ref_name() else {
        return vec![];
    };
    let ctx = ResolutionContext {
        path,
        is_completion: false,
    };
    let path_kind = path_kind(ctx.path.clone(), false);
    tracing::debug!(path_kind = ?path_kind);

    let resolve_variants = get_path_resolve_variants(db, ctx, path_kind);
    tracing::debug!(resolve_variants = ?resolve_variants);

    resolve_variants.into_iter().filter_by_name(path_name.as_str()).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionContext {
    pub path: InFile<ast::Path>,
    pub is_completion: bool,
}

impl ResolutionContext {
    pub fn containing_module(&self) -> Option<ast::Module> {
        self.path.value.syntax().containing_module()
    }

    pub fn wrap_in_file<T: AstNode>(&self, node: T) -> InFile<T> {
        InFile::new(self.path.file_id, node)
    }
}

pub fn process_path_resolve_variants(
    ctx: ResolutionContext,
    path_kind: PathKind,
    processor: &impl Processor,
) -> ProcessingStatus {
    match path_kind {
        PathKind::Unknown => ProcessingStatus::Continue,
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
