use crate::DiagnosticsContext;
use base_db::SourceDatabase;
use ide_db::assist_context::LocalAssists;
use lang::hir_db;
use lang::nameres::fq_named_element::ItemFQNameOwner;
use lang::nameres::path_kind::path_kind;
use lang::nameres::scope::ScopeEntry;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::ast::syntax_factory::SyntaxFactory;
use syntax::files::InFile;
use syntax::syntax_editor::Element;
use syntax::{AstNode, TextRange, ast};

pub(crate) fn auto_import_fixes(
    ctx: &DiagnosticsContext<'_>,
    path: InFile<ast::Path>,
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
            path.syntax().text_range(),
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
    assists.add_fix(
        "add-import",
        format!("Add import for `{}`", candidate_fq_name.fq_identifier_text()),
        reference_range,
        |editor| {
            if let Some(candidate_path) = candidate_named_element.value.use_path(editor) {
                let make = SyntaxFactory::new();
                if let Some((anchor, has_extra_newline_at_the_end)) =
                    current_use_items_owner.pos_after_last_use_stmt()
                {
                    let use_stmt = make.use_stmt(candidate_path);
                    let mut elements_to_add = vec![
                        make.newline().into(),
                        make.whitespace("    ").into(),
                        use_stmt.syntax().syntax_element(),
                    ];
                    if !has_extra_newline_at_the_end {
                        elements_to_add.push(make.newline().into());
                    }
                    editor.insert_all(anchor, elements_to_add);
                }
                editor.add_mappings(make.finish_with_mappings());
            }
        },
    );
    Some(())
}
