use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use lang::hir_db;
use lang::item_scope::NamedItemScope;
use lang::loc::SyntaxLocNodeExt;
use lang::nameres::use_speck_entries::{UseItem, UseItemType, use_items_for_stmt};
use std::collections::HashSet;
use syntax::ast::HasUseStmts;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};

pub(crate) fn find_unused_imports(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    use_stmts_owner: InFile<impl HasUseStmts>,
) -> Option<()> {
    for item_scope in vec![NamedItemScope::Main, NamedItemScope::Verify, NamedItemScope::Test] {
        find_unused_imports_for_item_scope(acc, ctx, use_stmts_owner.clone(), item_scope);
    }
    Some(())
}

fn find_unused_imports_for_item_scope(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    use_stmts_owner: InFile<impl HasUseStmts>,
    item_scope: NamedItemScope,
) -> Option<()> {
    let (file_id, use_stmts_owner) = use_stmts_owner.unpack();

    let mut reachable_paths = vec![];
    for path in use_stmts_owner.syntax().descendants_of_type::<ast::Path>() {
        if path.base_path() != path {
            continue;
        }
        if path.syntax().has_ancestor_strict::<ast::UseSpeck>() {
            continue;
        }
        if hir_db::item_scope(ctx.sema.db, path.loc(file_id)) != item_scope {
            continue;
        }
        reachable_paths.push(path);
    }

    let mut use_items_hit = HashSet::new();
    for path in reachable_paths {
        let base_path_type = BasePathType::for_path(&path);
        if base_path_type.is_none() {
            // fq path
            continue;
        }
        let base_path_type = base_path_type.unwrap();
        let use_item_owners = path.syntax().ancestors_of_type::<ast::AnyHasUseStmts>(true);
        for use_item_owner in use_item_owners {
            let mut reachable_use_items =
                hir_db::use_items(ctx.sema.db, use_item_owner.in_file(file_id))
                    .into_iter()
                    .filter(|it| it.scope == item_scope);
            let use_item_hit = match &base_path_type {
                BasePathType::Item { item_name } => reachable_use_items
                    .find(|it| it.type_ == UseItemType::Item && it.alias_or_name.eq(item_name)),
                BasePathType::Module { module_name } => reachable_use_items.find(|it| {
                    matches!(it.type_, UseItemType::Module | UseItemType::SelfModule)
                        && it.alias_or_name.eq(module_name)
                }),
                _ => None,
            };
            if let Some(use_item_hit) = use_item_hit {
                use_items_hit.insert(use_item_hit);
                break;
            }
        }
    }

    let use_stmts = use_stmts_owner.use_stmts();
    for use_stmt in use_stmts
        .into_iter()
        .filter(|it| hir_db::item_scope(ctx.sema.db, it.loc(file_id)) == item_scope)
    {
        check_unused_use_speck(acc, ctx, use_stmt.in_file(file_id), &use_items_hit);
    }

    None
}

fn check_unused_use_speck(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    use_stmt: InFile<ast::UseStmt>,
    use_items_hit: &HashSet<UseItem>,
) -> Option<()> {
    let mut unused_use_items = use_items_for_stmt(ctx.sema.db, use_stmt.clone())?;
    unused_use_items.retain(|it| !use_items_hit.contains(it));

    let module_use_items = unused_use_items.iter().find(|it| it.type_ == UseItemType::Module);
    if module_use_items.is_some() {
        acc.push(
            Diagnostic::new(
                DiagnosticCode::Lsp("unused-import", Severity::Warning),
                "Unused use item",
                use_stmt.file_range(),
            ),
            // .with_unused(true)
        );
        return Some(());
    }

    let use_items = use_items_for_stmt(ctx.sema.db, use_stmt.clone())?;
    if use_items.len() == unused_use_items.len() {
        // all inner speck types are covered, highlight complete useStmt
        acc.push(
            Diagnostic::new(
                DiagnosticCode::Lsp("unused-import", Severity::Warning),
                "Unused use item",
                use_stmt.file_range(),
            ),
            // .with_unused(true)
        );
    } else {
        for use_item in unused_use_items {
            acc.push(
                Diagnostic::new(
                    DiagnosticCode::Lsp("unused-import", Severity::Warning),
                    "Unused use item",
                    use_item.use_speck_loc.file_range(),
                ),
                // .with_unused(true)
            );
        }
    }
    Some(())
}

pub(crate) enum BasePathType {
    Address,
    Module { module_name: String },
    Item { item_name: String },
}

impl BasePathType {
    pub(crate) fn for_path(path: &ast::Path) -> Option<BasePathType> {
        let root_path = path.root_path();
        let qualifier = root_path.qualifier();
        match qualifier {
            // foo
            None => Some(BasePathType::Item {
                item_name: root_path.reference_name()?,
            }),
            // 0x1::foo
            Some(qualifier) if qualifier.path_address().is_some() => Some(BasePathType::Address),
            // m::foo
            Some(qualifier) => Some(BasePathType::Module {
                module_name: qualifier.reference_name()?,
            }),
        }
    }
}
