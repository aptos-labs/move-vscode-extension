use crate::DiagnosticsContext;
use base_db::SourceDatabase;
use ide_db::assist_context::LocalAssists;
use lang::hir_db;
use lang::nameres::fq_named_element::ItemFQNameOwner;
use lang::nameres::path_kind::path_kind;
use lang::nameres::scope::ScopeEntry;
use syntax::ast::UseStmtsOwner;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::ast::syntax_factory::SyntaxFactory;
use syntax::files::InFile;
use syntax::syntax_editor::SyntaxEditor;
use syntax::{AstNode, TextRange, ast};

pub(crate) fn auto_import_fix(
    ctx: &DiagnosticsContext<'_>,
    path: InFile<ast::Path>,
    reference_range: TextRange,
) -> Option<LocalAssists> {
    // find scope entries with this name from all importable entries in all reachable modules
    let db = ctx.sema.db;
    let reference_name = path.value.reference_name()?;

    let (file_id, path) = path.unpack();

    let expected_ns = path_kind(db, path.qualifier(), &path, false)?.unqualified_ns()?;
    let import_candidates = hir_db::import_candidates(db, file_id)
        .iter()
        .filter(|it| expected_ns.contains(it.ns))
        .filter(|it| it.name == reference_name)
        .cloned()
        .collect::<Vec<_>>();

    // limit to 3 autofixes, otherwise just bail out
    if import_candidates.len() > 3 {
        return None;
    }

    let current_items_owner = path.syntax().containing_items_owner()?;
    let mut assists = ctx.local_assists_for_node(InFile::new(file_id, &path))?;

    for import_candidate in import_candidates {
        add_autoimport_fix_for_import_candidate(
            db,
            &mut assists,
            import_candidate,
            &current_items_owner,
            reference_range,
        );
    }
    Some(assists)
}

fn add_autoimport_fix_for_import_candidate(
    db: &dyn SourceDatabase,
    assists: &mut LocalAssists,
    import_candidate: ScopeEntry,
    current_use_items_owner: &ast::AnyHasItems,
    reference_range: TextRange,
) -> Option<()> {
    let candidate_named_element = import_candidate.cast_into::<ast::NamedElement>(db)?;
    let candidate_fq_name = candidate_named_element.fq_name(db)?;
    assists.add_fix_with_make(
        "add-import",
        format!("Add import for `{}`", candidate_fq_name.fq_identifier_text()),
        reference_range,
        add_import_for_named_element(current_use_items_owner, candidate_named_element.value),
    );
    Some(())
}

fn add_import_for_named_element(
    items_owner: &ast::AnyHasItems,
    named_element: ast::NamedElement,
) -> impl FnOnce(&mut SyntaxEditor, &SyntaxFactory) -> Option<()> {
    move |editor, make| {
        let (item_module_path, item_name_ref) = make.item_path(named_element)?;

        // try to find existing use stmt for the module path first
        let existing_use_stmt = items_owner
            .use_stmts()
            .filter(|it| {
                it.module_path()
                    .is_some_and(|use_mod_path| use_mod_path.syntax_eq(&item_module_path))
            })
            .last();
        if let Some(use_stmt) = existing_use_stmt {
            let new_name_ref = match item_name_ref {
                Some(item_name_ref) => item_name_ref,
                None => make.name_ref("Self"),
            };
            use_stmt.add_group_item((new_name_ref, None), editor);
            return Some(());
        }

        let make = SyntaxFactory::new();
        let use_speck_path = match item_name_ref {
            Some(item_name_ref) => {
                make.path_from_qualifier_and_name_ref(item_module_path, item_name_ref)
            }
            None => item_module_path,
        };
        let use_stmt = make.use_stmt(use_speck_path);

        items_owner.add_use_stmt(use_stmt, editor);

        Some(())
    }
}
