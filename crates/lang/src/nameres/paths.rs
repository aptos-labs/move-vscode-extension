use crate::db::HirDatabase;
use crate::files::OptionInFileExt;
use crate::loc::SyntaxLocExt;
use crate::nameres::name_resolution::{
    get_entries_from_walking_scopes, get_modules_as_entries, get_qualified_path_entries,
};
use crate::nameres::namespaces::Ns;
use crate::nameres::path_kind::{path_kind, PathKind, QualifiedKind};
use crate::nameres::scope::{ScopeEntry, ScopeEntryListExt};
use crate::node_ext::PathLangExt;
use crate::{loc, InFile, Name};
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxNodeExt;
use syntax::{ast, AstNode};

pub fn get_path_resolve_variants(
    db: &dyn HirDatabase,
    ctx: ResolutionContext,
    path_kind: PathKind,
) -> Vec<ScopeEntry> {
    match path_kind {
        PathKind::Unknown => vec![],
        PathKind::NamedAddress(_) | PathKind::ValueAddress(_) => {
            // no path resolution for named / value addresses
            vec![]
        }
        PathKind::NamedAddressOrUnqualifiedPath { ns, .. } | PathKind::Unqualified { ns } => {
            let mut entries = vec![];
            if ns.contains(Ns::MODULE) {
                if let Some(module) = ctx.containing_module().opt_in_file(ctx.path.file_id) {
                    // Self::call() as an expression
                    entries.push(ScopeEntry {
                        name: Name::new("Self"),
                        node_loc: module.loc(),
                        ns: Ns::MODULE,
                        scope_adjustment: None,
                    })
                }
            }
            entries.extend(get_entries_from_walking_scopes(db, ctx, ns));
            entries
        }

        PathKind::Qualified {
            kind: QualifiedKind::Module { address },
            ..
        } => get_modules_as_entries(db, ctx, address),

        PathKind::Qualified { qualifier, ns, .. } => get_qualified_path_entries(db, ctx, qualifier)
            .into_iter()
            .filter_by_ns(ns)
            .collect(),
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
    let context_element = path.clone();
    let ctx = ResolutionContext {
        path,
        is_completion: false,
    };
    let path_kind = path_kind(ctx.path.clone(), false);
    tracing::debug!(?path_kind);

    let entries = get_path_resolve_variants(db, ctx, path_kind);
    tracing::debug!(?entries);

    let entries_filtered_by_name = entries
        .into_iter()
        .filter_by_name(path_name.clone())
        .collect::<Vec<_>>();
    tracing::debug!(?path_name, ?entries_filtered_by_name);

    entries_filtered_by_name
        .into_iter()
        .filter_by_visibility(db, context_element)
        .collect()
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
