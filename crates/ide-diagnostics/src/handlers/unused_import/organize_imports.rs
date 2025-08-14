use crate::handlers::unused_import::{
    UnusedImportKind, find_unused_use_items_for_item_scope, unused_import_kind,
};
use base_db::SourceDatabase;
use ide_db::RootDatabase;
use ide_db::assist_context::LocalAssists;
use ide_db::assists::{Assist, AssistResolveStrategy};
use lang::item_scope::NamedItemScope;
use lang::loc::SyntaxLocFileExt;
use lang::{Semantics, hir_db};
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
    let stmts_owner_with_siblings =
        hir_db::use_stmts_owner_with_siblings(db, use_stmts_owner.clone().map_into());

    for item_scope in NamedItemScope::all() {
        let unused_use_items =
            find_unused_use_items_for_item_scope(db, &stmts_owner_with_siblings, item_scope)?;

        let use_stmts = use_stmts_owner.as_ref().flat_map(|it| it.use_stmts().collect());
        for use_stmt in use_stmts
            .into_iter()
            .filter(|stmt| hir_db::item_scope(db, stmt.loc()) == item_scope)
        {
            let unused_stmt_use_items = unused_use_items
                .iter()
                .filter(|it| use_stmt.loc().contains(&it.use_speck_loc))
                .collect::<Vec<_>>();
            let unused_import_kind = unused_import_kind(db, use_stmt.clone(), unused_stmt_use_items)?;
            delete_unused_use_items(db, use_stmt.value, unused_import_kind, editor);
        }
    }

    Some(())
}

fn delete_unused_use_items(
    db: &dyn SourceDatabase,
    use_stmt: ast::UseStmt,
    unused_import_kind: UnusedImportKind,
    editor: &mut SyntaxEditor,
) -> Option<()> {
    match &unused_import_kind {
        UnusedImportKind::UseStmt { use_stmt } => use_stmt.value.delete(editor),
        UnusedImportKind::UseSpeck { use_speck_locs } => {
            let use_specks = use_speck_locs
                .iter()
                .filter_map(|it| it.to_ast::<ast::UseSpeck>(db))
                .map(|it| it.value)
                .collect();
            use_stmt.delete_group_use_specks(use_specks, editor);
        }
    }
    Some(())
}
