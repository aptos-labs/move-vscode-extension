use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use base_db::SourceDatabase;
use ide_db::Severity;
use lang::hir_db;
use lang::item_scope::NamedItemScope;
use lang::loc::SyntaxLocFileExt;
use lang::nameres::use_speck_entries::{UseItem, UseItemType, use_items_for_stmt};
use std::collections::HashSet;
use syntax::ast::UseStmtsOwner;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::InFile;
use syntax::{AstNode, SyntaxNode, ast};

pub(crate) fn find_unused_imports(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    use_stmts_owner: InFile<ast::AnyUseStmtsOwner>,
) -> Option<()> {
    let _p = tracing::debug_span!("find_unused_imports").entered();
    if ctx.config.assists_only {
        // no assists
        return None;
    }
    // special-case frequent path
    if use_stmts_owner
        .value
        .cast_into::<ast::BlockExpr>()
        .is_some_and(|it| it.use_stmts().collect::<Vec<_>>().is_empty())
    {
        return Some(());
    }

    let db = ctx.sema.db;
    let stmts_owner_with_siblings =
        hir_db::use_stmts_owner_with_siblings(db, use_stmts_owner.clone().map_into());

    for item_scope in vec![NamedItemScope::Main, NamedItemScope::Verify, NamedItemScope::Test] {
        let unused_use_items =
            find_unused_use_items_for_item_scope(ctx.sema.db, &stmts_owner_with_siblings, item_scope);
        if let Some(unused_use_items) = unused_use_items {
            for use_stmt_owner in stmts_owner_with_siblings.iter() {
                let use_stmts = use_stmt_owner.as_ref().flat_map(|it| it.use_stmts().collect());
                for use_stmt in use_stmts
                    .into_iter()
                    .filter(|stmt| hir_db::item_scope(db, stmt.loc()) == item_scope)
                {
                    let stmt_use_items = unused_use_items
                        .iter()
                        .filter(|it| use_stmt.loc().contains(&it.use_speck_loc))
                        .collect::<Vec<_>>();
                    highlight_unused_use_items(db, acc, use_stmt, stmt_use_items);
                }
            }
        }
    }
    Some(())
}

fn find_unused_use_items_for_item_scope(
    db: &dyn SourceDatabase,
    stmts_owner_with_siblings: &Vec<InFile<ast::AnyUseStmtsOwner>>,
    item_scope: NamedItemScope,
) -> Option<Vec<UseItem>> {
    let reachable_paths = stmts_owner_with_siblings
        .iter()
        .flat_map(|it| {
            it.as_ref()
                .flat_map(|stmts_owner| descendant_paths(stmts_owner.syntax()).collect())
        })
        .filter(|it| hir_db::item_scope(db, it.loc()) == item_scope)
        .collect::<Vec<_>>();

    let mut use_items_hit = HashSet::new();
    for path in reachable_paths {
        let base_path_type = BasePathType::for_path(&path.value);
        if base_path_type.is_none() {
            // fq path
            continue;
        }
        let base_path_type = base_path_type.unwrap();

        let use_item_owner_ancestors = path
            .as_ref()
            .flat_map(|it| it.syntax().ancestors_of_type::<ast::AnyUseStmtsOwner>(true));
        for use_item_owner_ans in use_item_owner_ancestors {
            let mut reachable_use_items =
                hir_db::use_items_from_self_and_siblings(db, use_item_owner_ans)
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

    let mut all_unused_use_items = vec![];
    for use_stmt_owner in stmts_owner_with_siblings {
        let use_stmts = use_stmt_owner.as_ref().flat_map(|it| it.use_stmts().collect());
        for use_stmt in use_stmts
            .into_iter()
            .filter(|stmt| hir_db::item_scope(db, stmt.loc()) == item_scope)
        {
            for use_item in use_items_for_stmt(db, use_stmt.clone())? {
                if !use_items_hit.contains(&use_item) {
                    all_unused_use_items.push(use_item);
                }
            }
        }
    }

    Some(all_unused_use_items)
}

fn descendant_paths(node: &SyntaxNode) -> impl Iterator<Item = ast::Path> {
    node.descendants_of_type::<ast::Path>()
        .filter(|path| &path.base_path() == path)
        .filter(|path| !path.syntax().has_ancestor_strict::<ast::UseSpeck>())
}

fn highlight_unused_use_items(
    db: &dyn SourceDatabase,
    acc: &mut Vec<Diagnostic>,
    use_stmt: InFile<ast::UseStmt>,
    unused_use_items: Vec<&UseItem>,
) -> Option<()> {
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

    let use_items = use_items_for_stmt(db, use_stmt.clone())?;
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
