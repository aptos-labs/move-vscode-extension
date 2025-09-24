// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::handlers::unused_import::{UnusedUseKind, find_unused_use_items, unused_use_kind};
use base_db::SourceDatabase;
use ide_db::RootDatabase;
use ide_db::assist_context::LocalAssists;
use ide_db::assists::{Assist, AssistResolveStrategy};
use lang::Semantics;
use lang::loc::SyntaxLocFileExt;
use syntax::ast::UseStmtsOwner;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::syntax_editor::SyntaxEditor;
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
        let unused_import_kind = unused_use_kind(db, use_stmt.clone(), unused_stmt_use_items)?;
        organize_imports_in_use_stmt(db, use_stmt.value, unused_import_kind, editor);
    }

    Some(())
}

fn organize_imports_in_use_stmt(
    db: &dyn SourceDatabase,
    use_stmt: ast::UseStmt,
    unused_import_kind: UnusedUseKind,
    editor: &mut SyntaxEditor,
) -> Option<()> {
    match &unused_import_kind {
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
