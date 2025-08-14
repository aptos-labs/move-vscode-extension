pub mod organize_imports;

use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use base_db::SourceDatabase;
use ide_db::Severity;
use lang::hir_db;
use lang::item_scope::NamedItemScope;
use lang::loc::{SyntaxLoc, SyntaxLocFileExt};
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

    for item_scope in NamedItemScope::all() {
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
                    let unused_import_kind = unused_import_kind(db, use_stmt.clone(), stmt_use_items)?;
                    highlight_unused_use_items(
                        ctx,
                        &use_stmts_owner,
                        use_stmt.value,
                        acc,
                        unused_import_kind,
                    );
                }
            }
        }
    }
    Some(())
}

pub(crate) fn find_unused_use_items_for_item_scope(
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

pub(crate) enum UnusedImportKind {
    UseStmt { use_stmt: InFile<ast::UseStmt> },
    UseSpeck { use_speck_locs: Vec<SyntaxLoc> },
}

impl UnusedImportKind {
    pub fn use_stmt(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::UseStmt>> {
        match self {
            UnusedImportKind::UseStmt { use_stmt } => Some(use_stmt.clone()),
            UnusedImportKind::UseSpeck { use_speck_locs } => use_speck_locs
                .first()
                .and_then(|it| it.to_ast::<ast::UseSpeck>(db))?
                .and_then(|it| it.parent_use_group())?
                .and_then(|it| it.use_stmt()),
        }
    }
}

pub(crate) fn unused_import_kind(
    db: &dyn SourceDatabase,
    use_stmt: InFile<ast::UseStmt>,
    unused_stmt_use_items: Vec<&UseItem>,
) -> Option<UnusedImportKind> {
    let module_use_items = unused_stmt_use_items
        .iter()
        .find(|it| it.type_ == UseItemType::Module);
    // use 0x1::unused_m;
    if module_use_items.is_some() {
        return Some(UnusedImportKind::UseStmt { use_stmt });
    }

    let actual_stmt_use_items = use_items_for_stmt(db, use_stmt.clone())?;
    // use 0x1::m::{};
    if actual_stmt_use_items.is_empty() {
        return Some(UnusedImportKind::UseStmt { use_stmt });
    }

    // use 0x1::m::{unused_a, unused_b};
    if actual_stmt_use_items.len() == unused_stmt_use_items.len() {
        // all inner speck types are covered, highlight complete useStmt
        return Some(UnusedImportKind::UseStmt { use_stmt });
    }

    Some(UnusedImportKind::UseSpeck {
        use_speck_locs: unused_stmt_use_items
            .iter()
            .map(|it| it.use_speck_loc.clone())
            .collect(),
    })
}

fn highlight_unused_use_items(
    ctx: &DiagnosticsContext<'_>,
    use_stmts_owner: &InFile<ast::AnyUseStmtsOwner>,
    use_stmt: ast::UseStmt,
    acc: &mut Vec<Diagnostic>,
    unused_import_kind: UnusedImportKind,
) -> Option<()> {
    match unused_import_kind {
        UnusedImportKind::UseStmt { use_stmt } => {
            acc.push(
                Diagnostic::new(
                    DiagnosticCode::Lsp("unused-import", Severity::Warning),
                    "Unused use item",
                    use_stmt.file_range(),
                )
                .with_local_fix(ctx.local_fix(
                    use_stmts_owner.as_ref(),
                    "remove-unused-import",
                    "Remove unused use stmt",
                    use_stmt.file_range().range,
                    |editor| {
                        use_stmt.value.delete(editor);
                    },
                )),
            );
        }
        UnusedImportKind::UseSpeck { ref use_speck_locs } => {
            for use_speck_loc in use_speck_locs {
                let diag_range = use_speck_loc.file_range();
                if let Some(use_speck) = use_speck_loc.to_ast::<ast::UseSpeck>(ctx.sema.db) {
                    acc.push(
                        Diagnostic::new(
                            DiagnosticCode::Lsp("unused-import", Severity::Warning),
                            "Unused use item",
                            use_speck.file_range(),
                        )
                        .with_local_fix(ctx.local_fix(
                            use_stmts_owner.as_ref(),
                            "remove-unused-import",
                            "Remove unused use item",
                            diag_range.range,
                            |editor| {
                                use_stmt.delete_group_use_specks(vec![use_speck.value], editor);
                            },
                        )),
                    );
                }
            }
        }
    }
    Some(())
}

enum BasePathType {
    Address,
    Module { module_name: String },
    Item { item_name: String },
}

impl BasePathType {
    fn for_path(path: &ast::Path) -> Option<BasePathType> {
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
