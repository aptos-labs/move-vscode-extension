use crate::db::HirDatabase;
use crate::files::InFileVecExt;
use crate::nameres::address::Address;
use crate::nameres::namespaces::{NsSet, NsSetExt, MODULES};
use crate::nameres::node_ext::ModuleResolutionExt;
use crate::nameres::paths;
use crate::nameres::paths::ResolutionContext;
use crate::nameres::scope::{NamedItemsExt, ScopeEntry};
use crate::nameres::scope_entries_owner::get_entries_in_scope;
use crate::node_ext::ModuleLangExt;
use crate::{InFile, Name};
use base_db::SourceRootDatabase;
use parser::SyntaxKind;
use parser::SyntaxKind::{MODULE_SPEC, STMT_LIST};
use syntax::ast::{HasItemList, HasReference};
use syntax::{ast, AstNode, SyntaxNode};

pub struct ResolveScope {
    scope: InFile<SyntaxNode>,
    prev: Option<SyntaxNode>,
}

pub fn get_resolve_scopes(
    _db: &dyn HirDatabase,
    start_at: InFile<impl HasReference>,
) -> Vec<ResolveScope> {
    let mut scopes = vec![];

    let file_id = start_at.file_id;
    let mut opt_scope = start_at.value.syntax().parent();
    let mut prev = None;
    while let Some(scope) = opt_scope {
        scopes.push(ResolveScope {
            scope: InFile::new(file_id, scope.clone()),
            prev: prev.clone(),
        });

        if scope.kind() == SyntaxKind::MODULE {
            let module = ast::Module::cast(scope.clone()).unwrap();
            for module_item_spec in module.module_item_specs() {
                if let Some(module_item_spec_block) = module_item_spec.spec_block() {
                    let scope = module_item_spec_block.syntax().to_owned();
                    scopes.push(ResolveScope {
                        scope: InFile::new(file_id, scope),
                        prev: prev.clone(),
                    })
                }
            }
            // todo: all `spec MODULE {}` specs
        }

        if scope.kind() == MODULE_SPEC {
            // todo: resolve to module item, then add it as a next scope
        }

        let parent_scope = scope.parent();
        // skip StmtList to be able to use came_from in let stmts shadowing
        if scope.kind() != STMT_LIST {
            prev = Some(scope);
        }
        opt_scope = parent_scope;
    }

    scopes
}

pub fn get_entries_from_walking_scopes(
    db: &dyn HirDatabase,
    ctx: ResolutionContext,
    ns: NsSet,
) -> Vec<ScopeEntry> {
    let start_at = ctx.path.clone();
    let resolve_scopes = get_resolve_scopes(db, start_at);
    let mut entries = vec![];
    for ResolveScope { scope, prev } in resolve_scopes {
        let scope_entries = get_entries_in_scope(db, scope, prev);
        if scope_entries.is_empty() {
            continue;
        }
        // todo: shadowing between scopes
        for scope_entry in scope_entries {
            if !ns.contains_any_of(scope_entry.ns) {
                continue;
            }
            entries.push(scope_entry);
        }
    }
    entries
}

#[tracing::instrument(level = "debug", skip(db, ctx))]
pub fn get_modules_as_entries(
    db: &dyn HirDatabase,
    ctx: ResolutionContext,
    address: Address,
) -> Vec<ScopeEntry> {
    // get all files in the current package
    let file_id = ctx.path.file_id;
    let source_root_id = db.file_source_root(file_id);
    let source_root = db.source_root(source_root_id);
    let mut entries = vec![];
    for source_file_id in source_root.iter() {
        let source_file = db.parse(source_file_id).tree();
        let modules = source_file
            .all_modules()
            .filter(|m| m.address_equals_to(address.clone(), false))
            .collect::<Vec<_>>();
        entries.extend(modules.wrapped_in_file(source_file_id).to_entries());
    }
    entries
}

#[tracing::instrument(
    level = "debug",
    skip(db, ctx, qualifier),
    fields(qualifier = ?qualifier.syntax().text(), path = ?ctx.path.syntax_text()))]
pub fn get_qualified_path_entries(
    db: &dyn HirDatabase,
    ctx: ResolutionContext,
    qualifier: ast::Path,
) -> Vec<ScopeEntry> {
    let qualifier = ctx.wrap_in_file(qualifier);
    let qualifier_item = paths::resolve_single(db, qualifier.clone());
    if qualifier_item.is_none() {
        // qualifier can be an address
        return vec![];
    }
    let qualifier_item = qualifier_item.unwrap();
    let mut entries = vec![];
    match qualifier_item.node_loc.kind() {
        SyntaxKind::MODULE => {
            entries.push(ScopeEntry {
                name: Name::new("Self"),
                node_loc: qualifier_item.node_loc,
                ns: MODULES,
                scope_adjustment: None,
            });
            let module = qualifier_item
                .node_loc
                .cast::<ast::Module>(db.upcast())
                .unwrap();
            entries.extend(module.member_entries())
        }
        SyntaxKind::ENUM => {
            // todo
        }
        _ => {}
    }
    entries
}
