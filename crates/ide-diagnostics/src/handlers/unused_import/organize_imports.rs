// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::handlers::reduced_scope_import::find_use_items_with_redundant_main_scope;
use crate::handlers::unused_import::{
    UnusedUseKind, UnusedUseStmtKind, find_unused_use_items, unused_use_kind,
};
use base_db::SourceDatabase;
use ide_db::RootDatabase;
use ide_db::assist_context::LocalAssists;
use ide_db::assists::{Assist, AssistResolveStrategy};
use lang::Semantics;
use lang::loc::SyntaxLocFileExt;
use syntax::ast::UseStmtsOwner;
use syntax::ast::edit::AstNodeEdit;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::ast::syntax_factory::SyntaxFactory;
use syntax::files::{InFile, InFileExt};
use syntax::syntax_editor::{Element, SyntaxEditor};
use syntax::{AstNode, ast};
use vfs::FileId;

pub fn organize_imports_in_file(db: &RootDatabase, file_id: FileId) -> Option<Assist> {
    let sema = Semantics::new(db, file_id);

    let file = sema.parse(file_id);
    let mut assists = LocalAssists::new_for_file(file_id, file.clone(), AssistResolveStrategy::All)?;
    assists.add_fix(
        "organize-imports",
        "Organize Imports in file",
        file.syntax().text_range(),
        |editor| {
            let use_stmts_owners = file.syntax().descendants_of_type::<ast::AnyUseStmtsOwner>();
            for use_stmts_owner in use_stmts_owners {
                organize_imports_in_stmts_owner(db, use_stmts_owner.in_file(file_id), editor);
            }
        },
    );

    assists.assists().pop()
}

fn organize_imports_in_stmts_owner(
    db: &dyn SourceDatabase,
    use_stmts_owner: InFile<ast::AnyUseStmtsOwner>,
    editor: &mut SyntaxEditor,
) -> Option<()> {
    let unused_use_items = find_unused_use_items(db, &use_stmts_owner)?;
    let use_stmts = use_stmts_owner.as_ref().flat_map(|it| it.use_stmts());
    for use_stmt in use_stmts {
        let unused_stmt_use_items = unused_use_items
            .iter()
            .filter(|it| use_stmt.loc().contains(&it.use_speck_loc))
            .collect::<Vec<_>>();
        let unused_use_kind = unused_use_kind(db, use_stmt.clone(), unused_stmt_use_items)?;
        remove_unused_imports_in_use_stmt(db, use_stmt.value, unused_use_kind, editor);
    }

    let redundant_main_scope_use_items = find_use_items_with_redundant_main_scope(db, &use_stmts_owner)?;
    let use_stmts = use_stmts_owner.as_ref().flat_map(|it| it.use_stmts());
    for use_stmt in use_stmts {
        let redundant_scope_use_items = redundant_main_scope_use_items
            .iter()
            .filter(|it| use_stmt.loc().contains(&it.use_speck_loc))
            .collect::<Vec<_>>();
        let unused_use_kind = unused_use_kind(db, use_stmt.clone(), redundant_scope_use_items)?;
        replace_main_scope_with_test_only(db, use_stmt.value, unused_use_kind, editor);
    }

    Some(())
}

fn remove_unused_imports_in_use_stmt(
    db: &dyn SourceDatabase,
    use_stmt: ast::UseStmt,
    unused_use_kind: UnusedUseKind,
    editor: &mut SyntaxEditor,
) -> Option<()> {
    match &unused_use_kind {
        UnusedUseKind::UseStmt { use_stmt, .. } => use_stmt.value.delete(editor),
        UnusedUseKind::UseSpeck { use_speck_locs } => {
            if use_stmt.use_speck().is_some_and(|it| it.is_root_self()) {
                // try to simplify use stmt
                use_stmt.simplify_root_self(editor);
                return Some(());
            }
            let use_specks = use_speck_locs
                .iter()
                .filter_map(|it| it.to_ast::<ast::UseSpeck>(db))
                .map(|it| it.value)
                .collect();
            use_stmt.delete_group_use_specks(editor, use_specks);
        }
    }
    Some(())
}

fn replace_main_scope_with_test_only(
    db: &dyn SourceDatabase,
    use_stmt: ast::UseStmt,
    unused_use_kind: UnusedUseKind,
    editor: &mut SyntaxEditor,
) -> Option<()> {
    match unused_use_kind {
        UnusedUseKind::UseStmt { use_stmt, kind } => {
            if kind == UnusedUseStmtKind::EmptyGroup {
                return None;
            }
            use_stmt.value.add_attribute(editor, "test_only");
        }
        UnusedUseKind::UseSpeck { use_speck_locs } => {
            let root_use_speck = use_stmt.use_speck()?;
            let module_path = root_use_speck.path()?;
            for use_speck_loc in use_speck_locs {
                if let Some(use_speck) = use_speck_loc.to_ast::<ast::UseSpeck>(db).map(|it| it.value)
                    && let Some(use_speck_name_ref) = use_speck.path_name_ref()
                {
                    let use_speck_alias = use_speck.use_alias().clone();
                    use_stmt.delete_group_use_specks(editor, vec![use_speck.clone()]);

                    let make = SyntaxFactory::new();
                    let new_root_use_speck = make.root_use_speck(
                        module_path.clone(),
                        Some(use_speck_name_ref.clone()),
                        use_speck_alias,
                    );
                    editor.insert_at_next_line_after(
                        &use_stmt,
                        make.use_stmt(vec![make.attr("test_only")], new_root_use_speck)
                            .indent_inner(use_stmt.indent_level())
                            .syntax()
                            .syntax_element(),
                    );

                    editor.add_mappings(make.finish_with_mappings());
                }
            }
        }
    }
    Some(())
}
