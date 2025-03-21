use crate::db::HirDatabase;
use crate::files::{InFileInto, InFileVecExt};
use crate::nameres::address::{Address, NamedAddr};
use crate::nameres::namespaces::{Ns, NsSet};
use crate::nameres::node_ext::ModuleResolutionExt;
use crate::nameres::paths::ResolutionContext;
use crate::nameres::scope::{NamedItemsExt, NamedItemsInFileExt, ScopeEntry};
use crate::nameres::scope_entries_owner::get_entries_in_scope;
use crate::node_ext::{ModuleLangExt, PathLangExt};
use crate::InFile;
use parser::SyntaxKind;
use parser::SyntaxKind::MODULE_SPEC;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use syntax::ast::{HasFields, HasItems, ReferenceElement};
use syntax::{ast, AstNode, SyntaxNode};

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

pub fn get_resolve_scopes(_db: &dyn HirDatabase, start_at: InFile<impl ReferenceElement>) -> Vec<ResolveScope> {
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
        prev = Some(scope);
        // skip StmtList to be able to use came_from in let stmts shadowing
        // if scope.kind() != STMT_LIST {
        // }
        opt_scope = parent_scope;
    }

    scopes
}

pub fn get_entries_from_walking_scopes(
    db: &dyn HirDatabase,
    ctx: &ResolutionContext,
    ns: NsSet,
) -> Vec<ScopeEntry> {
    let start_at = ctx.path.clone();
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

#[tracing::instrument(level = "debug", skip(db, ctx, address), fields(path = ctx.path.syntax_text()))]
pub fn get_modules_as_entries(
    db: &dyn HirDatabase,
    ctx: &ResolutionContext,
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
    ctx: &ResolutionContext,
    qualifier: ast::Path,
) -> Vec<ScopeEntry> {
    let qualifier = ctx.wrap_in_file(qualifier);
    let qualifier_item = db.resolve_path(qualifier.clone());
    if qualifier_item.is_none() {
        // qualifier can be an address
        if let Some(qualifier_name) = qualifier.value.reference_name() {
            return get_modules_as_entries(db, ctx, Address::Named(NamedAddr::new(qualifier_name)));
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
                node_loc: qualifier_item.node_loc,
                ns: Ns::MODULE,
                scope_adjustment: None,
            });
            let module = qualifier_item
                .node_loc
                .cast_into::<ast::Module>(db.upcast())
                .unwrap();
            entries.extend(module.member_entries())
        }
        SyntaxKind::ENUM => {
            let enum_ = qualifier_item
                .node_loc
                .cast_into::<ast::Enum>(db.upcast())
                .unwrap();
            entries.extend(enum_.value.variants().to_in_file_entries(enum_.file_id));
        }
        _ => {}
    }
    entries
}

pub fn get_struct_pat_field_resolve_variants(
    db: &dyn HirDatabase,
    struct_pat_field: InFile<ast::StructPatField>,
) -> Vec<ScopeEntry> {
    let struct_pat_path = struct_pat_field.map(|field| field.struct_pat().path());
    db.resolve_path(struct_pat_path)
        .and_then(|struct_entry| {
            let fields_owner = struct_entry
                .node_loc
                .cast_into::<ast::AnyHasFields>(db.upcast())?;
            Some(get_named_field_entries(fields_owner))
        })
        .unwrap_or_default()
}

pub fn get_struct_lit_field_resolve_variants(
    db: &dyn HirDatabase,
    struct_lit_field: InFile<ast::StructLitField>,
) -> Vec<ScopeEntry> {
    let struct_lit_path = struct_lit_field.map(|field| field.struct_lit().path());
    db.resolve_path(struct_lit_path)
        .and_then(|struct_entry| {
            let fields_owner = struct_entry
                .node_loc
                .cast_into::<ast::AnyHasFields>(db.upcast())?;
            Some(get_named_field_entries(fields_owner))
        })
        .unwrap_or_default()
}

pub fn get_named_field_entries(fields_owner: InFile<ast::AnyHasFields>) -> Vec<ScopeEntry> {
    fields_owner
        .value
        .named_fields()
        .to_in_file_entries(fields_owner.file_id)
}
