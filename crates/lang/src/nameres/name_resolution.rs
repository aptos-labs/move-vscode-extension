use crate::hir_db::get_modules_in_file;
use crate::nameres::address::Address;
use crate::nameres::namespaces::{Ns, NsSet};
use crate::nameres::node_ext::ModuleResolutionExt;
use crate::nameres::path_resolution::ResolutionContext;
use crate::nameres::scope::{NamedItemsExt, NamedItemsInFileExt, ScopeEntry};
use crate::nameres::scope_entries_owner::get_entries_in_scope;
use crate::node_ext::item::ModuleItemExt;
use crate::{hir_db, nameres};
use base_db::SourceDatabase;
use base_db::package_root::PackageId;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use syntax::SyntaxKind;
use syntax::SyntaxKind::MODULE_SPEC;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::{InFile, InFileExt, InFileVecExt};
use syntax::{AstNode, SyntaxNode, ast};

pub struct ResolveScope {
    scope: InFile<SyntaxNode>,
    prev: SyntaxNode,
}

impl fmt::Debug for ResolveScope {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_set()
            .entry(&self.scope.value.kind())
            .entry(&self.prev.kind())
            .finish()
    }
}

pub fn get_resolve_scopes(db: &dyn SourceDatabase, start_at: InFile<SyntaxNode>) -> Vec<ResolveScope> {
    let (file_id, start_at) = start_at.unpack();

    let mut scopes = vec![];
    let mut opt_scope = start_at.parent();
    let mut prev_scope = start_at.to_owned();

    while let Some(ref scope) = opt_scope {
        scopes.push(ResolveScope {
            scope: InFile::new(file_id, scope.clone()),
            prev: prev_scope.clone(),
        });

        if scope.kind() == SyntaxKind::MODULE {
            let module = ast::Module::cast(scope.clone()).unwrap().in_file(file_id);
            scopes.extend(module_inner_spec_scopes(module.clone(), prev_scope));

            let prev = module.value.syntax().clone();
            for related_module_spec in module.related_module_specs(db) {
                scopes.push(ResolveScope {
                    prev: prev.clone(),
                    scope: related_module_spec.syntax(),
                });
            }
            break;
        }

        if scope.kind() == MODULE_SPEC {
            let module_spec = scope.clone().cast::<ast::ModuleSpec>().unwrap();
            if module_spec.path().is_none_or(|it| it.syntax() == &prev_scope) {
                // skip if we're resolving module path for the module spec
                break;
            }
            if let Some(module) = module_spec.clone().in_file(file_id).module(db) {
                let prev = module_spec.syntax().clone();
                scopes.push(ResolveScope {
                    scope: module.clone().map(|it| it.syntax().clone()),
                    prev: prev.clone(),
                });
                scopes.extend(module_inner_spec_scopes(module, prev.clone()));
            }
            break;
        }

        let parent_scope = scope.parent();
        prev_scope = scope.clone();
        opt_scope = parent_scope;
    }

    scopes
}

// all `spec module {}` in item container
fn module_inner_spec_scopes(
    item_container: InFile<impl ast::HasItems>,
    prev: SyntaxNode,
) -> Vec<ResolveScope> {
    let (file_id, module) = item_container.unpack();
    let mut inner_scopes = vec![];
    for module_item_spec in module.module_item_specs() {
        if let Some(module_item_spec_block) = module_item_spec.spec_block() {
            let scope = module_item_spec_block.syntax().to_owned();
            inner_scopes.push(ResolveScope {
                scope: InFile::new(file_id, scope),
                prev: prev.clone(),
            })
        }
    }
    inner_scopes
}

pub fn get_entries_from_walking_scopes(
    db: &dyn SourceDatabase,
    start_at: InFile<SyntaxNode>,
    ns: NsSet,
) -> Vec<ScopeEntry> {
    let _p = tracing::debug_span!("get_entries_from_walking_scopes").entered();

    let resolve_scopes = get_resolve_scopes(db, start_at);

    let mut visited_names = HashMap::<String, NsSet>::new();
    let mut entries = vec![];
    for ResolveScope { scope, prev } in resolve_scopes {
        let scope_entries = get_entries_in_scope(db, scope.clone(), prev);
        if scope_entries.is_empty() {
            continue;
        }
        let mut visited_names_in_scope = HashMap::<String, NsSet>::new();
        for scope_entry in scope_entries {
            let entry_name = scope_entry.name.clone();
            let entry_ns = scope_entry.ns;

            if !ns.contains(entry_ns) {
                continue;
            }

            if let Some(visited_ns) = visited_names.get(&entry_name) {
                if visited_ns.contains(entry_ns) {
                    // this (name, ns) is already visited in the previous scope
                    continue;
                }
            }

            let combined_ns = visited_names
                .get(&entry_name)
                .cloned()
                .unwrap_or_default()
                .union(NsSet::from(entry_ns));
            visited_names_in_scope.insert(entry_name, combined_ns);

            entries.push(scope_entry);
        }
        visited_names.extend(visited_names_in_scope);
    }
    entries
}

#[tracing::instrument(level = "debug", skip_all)]
pub fn get_modules_as_entries(
    db: &dyn SourceDatabase,
    package_id: PackageId,
    address: Address,
) -> Vec<ScopeEntry> {
    let interesting_file_ids = hir_db::file_ids_by_module_address(db, package_id, address.clone());
    tracing::debug!(?interesting_file_ids);

    let mut module_entries = vec![];
    for source_file_id in interesting_file_ids {
        let modules = get_modules_in_file(db, source_file_id, address.clone());
        module_entries.extend(modules.wrapped_in_file(source_file_id).to_entries());
    }
    tracing::debug!(?module_entries);

    module_entries
}

#[tracing::instrument(
    level = "debug",
    skip(db, ctx, qualifier),
    fields(qualifier = ?qualifier.syntax().text(), path = ?ctx.start_at.value.text()))]
pub fn get_qualified_path_entries(
    db: &dyn SourceDatabase,
    ctx: &ResolutionContext,
    qualifier: ast::Path,
) -> Vec<ScopeEntry> {
    let qualifier = ctx.wrap_in_file(qualifier);
    let qualifier_item = nameres::resolve_no_inf(db, qualifier.clone());
    if qualifier_item.is_none() {
        if let Some(qualifier_name) = qualifier.value.reference_name() {
            let _p = tracing::debug_span!(
                "qualifier is unresolved",
                "try to resolve assuming that {:?} is a named address",
                qualifier_name
            )
            .entered();
            return get_modules_as_entries(db, ctx.package_id(db), Address::named(&qualifier_name));
        }
        return vec![];
    }
    let qualifier_item = qualifier_item.unwrap();
    let mut entries = vec![];
    match qualifier_item.node_loc.kind() {
        SyntaxKind::MODULE => {
            // Self::call() as an expression
            entries.push(ScopeEntry {
                name: "Self".to_string(),
                node_loc: qualifier_item.node_loc.clone(),
                ns: Ns::MODULE,
                scope_adjustment: None,
            });
            entries.extend(hir_db::module_importable_entries(
                db,
                qualifier_item.node_loc.clone(),
            ));
            entries.extend(hir_db::module_importable_entries_from_related(
                db,
                qualifier_item.node_loc,
            ));
        }
        SyntaxKind::ENUM => {
            let Some(enum_) = qualifier_item.node_loc.to_ast::<ast::Enum>(db) else {
                return vec![];
            };
            entries.extend(enum_.value.variants().to_entries(enum_.file_id));
        }
        _ => {}
    }
    entries
}
