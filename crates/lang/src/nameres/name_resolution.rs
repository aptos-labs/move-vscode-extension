use crate::db::HirDatabase;
use crate::nameres::ResolveReference;
use crate::nameres::address::Address;
use crate::nameres::namespaces::{Ns, NsSet};
use crate::nameres::node_ext::ModuleResolutionExt;
use crate::nameres::path_resolution::ResolutionContext;
use crate::nameres::scope::{NamedItemsExt, NamedItemsInFileExt, ScopeEntry};
use crate::nameres::scope_entries_owner::get_entries_in_scope;
use crate::node_ext::ModuleLangExt;
use crate::node_ext::item::ModuleItemExt;
use base_db::package_root::PackageRootId;
use itertools::Itertools;
use parser::SyntaxKind;
use parser::SyntaxKind::MODULE_SPEC;
use std::collections::{HashMap, HashSet};
use std::fmt::Formatter;
use std::ops::Deref;
use std::{fmt, iter};
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxNodeExt;
use syntax::ast::{HasItems, ReferenceElement};
use syntax::files::{InFile, InFileExt, InFileVecExt};
use syntax::{AstNode, SyntaxNode, ast};

pub struct ResolveScope {
    scope: InFile<SyntaxNode>,
    prev: Option<SyntaxNode>,
}

impl fmt::Debug for ResolveScope {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_set()
            .entry(&self.scope.value.kind())
            .entry(&self.prev.clone().map(|it| it.kind()))
            .finish()
    }
}

pub fn get_resolve_scopes(
    db: &dyn HirDatabase,
    start_at: InFile<impl ReferenceElement>,
) -> Vec<ResolveScope> {
    let mut scopes = vec![];

    let file_id = start_at.file_id;
    let mut opt_scope = start_at.value.syntax().parent();
    let mut prev = None;
    while let Some(ref scope) = opt_scope {
        scopes.push(ResolveScope {
            scope: InFile::new(file_id, scope.clone()),
            prev: prev.clone(),
        });

        if scope.kind() == SyntaxKind::MODULE {
            let module = ast::Module::cast(scope.clone()).unwrap().in_file(file_id);
            scopes.extend(module_inner_spec_scopes(module, prev));
            break;
            // todo: all `spec MODULE {}` specs
        }

        if scope.kind() == MODULE_SPEC {
            let module_spec = scope.clone().cast::<ast::ModuleSpec>().unwrap();
            if prev == module_spec.path().map(|it| it.syntax().clone()) {
                // skip if we're resolving module path for the module spec
                break;
            }
            if let Some(module) = module_spec.clone().in_file(file_id).module(db) {
                let prev = Some(module_spec.syntax().clone());
                scopes.push(ResolveScope {
                    scope: module.clone().map(|it| it.syntax().clone()),
                    prev: prev.clone(),
                });
                scopes.extend(module_inner_spec_scopes(module, prev.clone()));
            }
            break;
        }

        let parent_scope = scope.parent();
        prev = Some(scope.clone());
        opt_scope = parent_scope;
    }

    scopes
}

fn module_inner_spec_scopes(module: InFile<ast::Module>, prev: Option<SyntaxNode>) -> Vec<ResolveScope> {
    let (file_id, module) = module.unpack();
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
    db: &dyn HirDatabase,
    start_at: InFile<impl ReferenceElement>,
    ns: NsSet,
) -> Vec<ScopeEntry> {
    let resolve_scopes = get_resolve_scopes(db, start_at);

    let mut visited_name_ns = HashMap::<String, NsSet>::new();
    let mut entries = vec![];
    for ResolveScope { scope, prev } in resolve_scopes {
        let scope_entries = get_entries_in_scope(db, scope, prev);
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

            if let Some(visited_ns) = visited_name_ns.get(&entry_name) {
                if visited_ns.contains(entry_ns) {
                    // this (name, ns) is already visited in the previous scope
                    continue;
                }
            }

            let old_ns = visited_names_in_scope.entry(entry_name).or_insert(NsSet::empty());
            *old_ns = *old_ns | NsSet::from(entry_ns);

            entries.push(scope_entry);
        }
        visited_name_ns.extend(visited_names_in_scope);
    }
    entries
}

#[tracing::instrument(level = "debug", skip(db))]
pub fn get_modules_as_entries(
    db: &dyn HirDatabase,
    package_root_id: PackageRootId,
    address: Address,
) -> Vec<ScopeEntry> {
    let source_file_ids = db.source_file_ids(package_root_id);

    let mut module_entries = vec![];
    for source_file_id in source_file_ids {
        let source_file = db.parse(source_file_id).tree();
        let modules = source_file
            .all_modules()
            .filter(|m| m.address_equals_to(address.clone(), false))
            .collect::<Vec<_>>();
        module_entries.extend(modules.wrapped_in_file(source_file_id).to_entries());
    }
    tracing::debug!(?module_entries);
    module_entries
}

#[tracing::instrument(
    level = "debug",
    skip(db, ctx, qualifier),
    fields(qualifier = ?qualifier.syntax().text(), path = ?ctx.path.syntax_text()))]
pub fn get_qualified_path_entries(
    db: &dyn HirDatabase,
    ctx: &ResolutionContext,
    qualifier: ast::Path,
) -> Option<Vec<ScopeEntry>> {
    let qualifier = ctx.wrap_in_file(qualifier);
    let qualifier_item = qualifier.clone().resolve_no_inf(db);
    if qualifier_item.is_none() {
        if let Some(qualifier_name) = qualifier.value.reference_name() {
            let _p = tracing::debug_span!(
                "qualifier is unresolved",
                "try to resolve assuming that {:?} is a named address",
                qualifier_name
            )
            .entered();

            return Some(get_modules_as_entries(
                db,
                ctx.package_root_id(db),
                Address::named(&qualifier_name),
            ));
        }
        return None;
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
            let module = qualifier_item.node_loc.to_ast::<ast::Module>(db.upcast())?;
            entries.extend(module.member_entries())
        }
        SyntaxKind::ENUM => {
            let enum_ = qualifier_item.node_loc.to_ast::<ast::Enum>(db.upcast())?;
            entries.extend(enum_.value.variants().to_in_file_entries(enum_.file_id));
        }
        _ => {}
    }
    Some(entries)
}
