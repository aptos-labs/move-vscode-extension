use crate::db::HirDatabase;
use crate::nameres::namespaces::{NsSet, NsSetExt};
use crate::nameres::scope_entries_owner::get_entries_in_scope;
use crate::nameres::scope::ScopeEntry;
use parser::SyntaxKind::{MODULE, MODULE_SPEC, STMT_LIST};
use syntax::ast::{HasItemList, HasReference};
use syntax::{ast, AstNode, NodeOrToken, SyntaxNode, SyntaxToken};

pub struct ResolveScope {
    scope: SyntaxNode,
    prev: Option<SyntaxNode>,
}

pub fn get_resolve_scopes(start_at: impl HasReference) -> Vec<ResolveScope> {
    let mut scopes = vec![];

    let mut opt_scope = start_at.syntax().parent();
    let mut prev = None;
    while let Some(scope) = opt_scope {
        scopes.push(ResolveScope {
            scope: scope.clone(),
            prev: prev.clone(),
        });

        if scope.kind() == MODULE {
            let module = ast::Module::cast(scope.clone()).unwrap();
            for module_item_spec in module.module_item_specs() {
                if let Some(module_item_spec_block) = module_item_spec.spec_block() {
                    scopes.push(ResolveScope {
                        scope: module_item_spec_block.syntax().to_owned(),
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
    start_at: impl HasReference,
    ns: NsSet,
) -> Vec<ScopeEntry> {
    let resolve_scopes = get_resolve_scopes(start_at);
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

