// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod organize_imports;
pub(crate) mod use_items;

use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::handlers::reduced_scope_import::find_use_items_with_redundant_main_scope;
use base_db::SourceDatabase;
use ide_db::Severity;
use lang::hir_db;
use lang::loc::{SyntaxLoc, SyntaxLocFileExt};
use lang::nameres::use_speck_entries::{UseItem, UseItemType, use_items_for_stmt};
use std::collections::HashSet;
use syntax::ast::UseStmtsOwner;
use syntax::ast::edit::AstNodeEdit;
use syntax::ast::syntax_factory::SyntaxFactory;
use syntax::files::InFile;
use syntax::syntax_editor::Element;
use syntax::{AstNode, ast};

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

    let unused_use_items_in_scope = find_unused_use_items(db, &use_stmts_owner).unwrap_or_default();
    for use_stmt in hir_db::combined_use_stmts(db, &use_stmts_owner) {
        let stmt_use_items = unused_use_items_in_scope
            .iter()
            .filter(|it| use_stmt.loc().contains(&it.use_speck_loc))
            .collect::<Vec<_>>();
        let unused_import_kind = unused_use_kind(db, use_stmt.clone(), stmt_use_items)?;
        highlight_unused_use_items(ctx, &use_stmts_owner, use_stmt.value, acc, unused_import_kind);
    }

    let unused_scoped_use_items =
        find_use_items_with_redundant_main_scope(db, &use_stmts_owner).unwrap_or_default();
    for use_stmt in hir_db::combined_use_stmts(db, &use_stmts_owner) {
        let unused_use_items_for_current_stmt = unused_scoped_use_items
            .iter()
            .filter(|it| use_stmt.loc().contains(&it.use_speck_loc))
            .collect::<Vec<_>>();
        let unused_use_kind = unused_use_kind(db, use_stmt.clone(), unused_use_items_for_current_stmt)?;
        highlight_unused_scoped_use_items(ctx, &use_stmts_owner, use_stmt.value, acc, unused_use_kind);
    }

    Some(())
}

pub(crate) fn find_unused_use_items(
    db: &dyn SourceDatabase,
    use_stmts_owner: &InFile<ast::AnyUseStmtsOwner>,
) -> Option<Vec<UseItem>> {
    let _p = tracing::debug_span!("find_unused_use_items").entered();

    let use_items_hit = use_items::find_use_items_hit_with_scopes(db, &use_stmts_owner)
        .into_iter()
        .map(|it| it.0)
        .collect::<HashSet<_>>();

    let mut all_unused_use_items = vec![];
    for use_stmt in hir_db::combined_use_stmts(db, &use_stmts_owner) {
        for use_item in use_items_for_stmt(db, use_stmt)? {
            if !use_items_hit.contains(&use_item) {
                all_unused_use_items.push(use_item);
            }
        }
    }

    Some(all_unused_use_items)
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum UnusedUseStmtKind {
    Module,
    EmptyGroup,
    AllSpecksInGroup,
}

#[derive(Debug)]
pub(crate) enum UnusedUseKind {
    UseStmt {
        use_stmt: InFile<ast::UseStmt>,
        kind: UnusedUseStmtKind,
    },
    UseSpeck {
        use_speck_locs: Vec<SyntaxLoc>,
    },
}

impl UnusedUseKind {
    pub fn use_stmt(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::UseStmt>> {
        match self {
            UnusedUseKind::UseStmt { use_stmt, .. } => Some(use_stmt.clone()),
            UnusedUseKind::UseSpeck { use_speck_locs } => use_speck_locs
                .first()
                .and_then(|it| it.to_ast::<ast::UseSpeck>(db))?
                .and_then(|it| it.parent_use_group())?
                .and_then(|it| it.use_stmt()),
        }
    }
}

pub(crate) fn unused_use_kind(
    db: &dyn SourceDatabase,
    use_stmt: InFile<ast::UseStmt>,
    unused_stmt_use_items: Vec<&UseItem>,
) -> Option<UnusedUseKind> {
    let module_use_items = unused_stmt_use_items
        .iter()
        .find(|it| it.type_ == UseItemType::Module);

    // use 0x1::unused_m;
    if module_use_items.is_some() {
        return Some(UnusedUseKind::UseStmt {
            use_stmt,
            kind: UnusedUseStmtKind::Module,
        });
    }

    let actual_stmt_use_items = use_items_for_stmt(db, use_stmt.clone())?;
    // use 0x1::m::{};
    if actual_stmt_use_items.is_empty() {
        return Some(UnusedUseKind::UseStmt {
            use_stmt,
            kind: UnusedUseStmtKind::EmptyGroup,
        });
    }

    // use 0x1::m::{unused_a, unused_b};
    if actual_stmt_use_items.len() == unused_stmt_use_items.len() {
        // all inner speck types are covered, highlight complete useStmt
        return Some(UnusedUseKind::UseStmt {
            use_stmt,
            kind: UnusedUseStmtKind::AllSpecksInGroup,
        });
    }

    Some(UnusedUseKind::UseSpeck {
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
    unused_import_kind: UnusedUseKind,
) -> Option<()> {
    match unused_import_kind {
        UnusedUseKind::UseStmt { use_stmt, .. } => {
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
        UnusedUseKind::UseSpeck { ref use_speck_locs } => {
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
                                use_stmt.delete_group_use_specks(editor, vec![use_speck.value]);
                            },
                        )),
                    );
                }
            }
        }
    }
    Some(())
}

fn highlight_unused_scoped_use_items(
    ctx: &DiagnosticsContext<'_>,
    use_stmts_owner: &InFile<ast::AnyUseStmtsOwner>,
    use_stmt: ast::UseStmt,
    acc: &mut Vec<Diagnostic>,
    unused_use_kind: UnusedUseKind,
) -> Option<()> {
    match unused_use_kind {
        UnusedUseKind::UseStmt { use_stmt, kind } => {
            // skip use stmts with empty groups
            if kind == UnusedUseStmtKind::EmptyGroup {
                return None;
            }
            acc.push(
                Diagnostic::new(
                    DiagnosticCode::Lsp("too-broad-scoped-import", Severity::Warning),
                    "Use item is used only in test scope and should be declared as #[test_only]",
                    use_stmt.file_range(),
                )
                .with_local_fix(ctx.local_fix(
                    use_stmts_owner.as_ref(),
                    "add-scope-attribute",
                    "Add #[test_only] attribute",
                    use_stmt.file_range().range,
                    |editor| {
                        use_stmt.value.add_attribute(editor, "test_only");
                    },
                )),
            );
        }
        UnusedUseKind::UseSpeck { ref use_speck_locs } => {
            let root_use_speck = use_stmt.use_speck()?;
            let module_path = root_use_speck.path()?;
            for use_speck_loc in use_speck_locs {
                let diag_range = use_speck_loc.file_range();
                if let Some(use_speck) = use_speck_loc
                    .to_ast::<ast::UseSpeck>(ctx.sema.db)
                    .map(|it| it.value)
                    && let Some(use_speck_name_ref) = use_speck.path_name_ref()
                {
                    let use_speck_alias = use_speck.use_alias().clone();
                    acc.push(
                        Diagnostic::new(
                            DiagnosticCode::Lsp("too-broad-scoped-import", Severity::Warning),
                            "Use item is used only in test scope and should be declared as #[test_only]",
                            diag_range,
                        )
                        .with_local_fix(ctx.local_fix(
                            use_stmts_owner.as_ref(),
                            "add-scope-attribute",
                            "Add #[test_only] attribute",
                            diag_range.range,
                            |editor| {
                                use_stmt.delete_group_use_specks(editor, vec![use_speck.clone()]);
                                let make = SyntaxFactory::new();
                                let indent_level = use_stmt.indent_level();
                                let new_root_use_speck = make.root_use_speck(
                                    module_path.clone(),
                                    Some(use_speck_name_ref.clone()),
                                    use_speck_alias,
                                );
                                editor.insert_at_next_line_after(
                                    &use_stmt,
                                    make.use_stmt(vec![make.attr("test_only")], new_root_use_speck)
                                        .indent_inner(indent_level)
                                        .syntax()
                                        .syntax_element(),
                                );
                                editor.add_mappings(make.finish_with_mappings());
                            },
                        )),
                    );
                }
            }
        }
    }
    Some(())
}
