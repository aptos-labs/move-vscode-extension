use crate::DiagnosticsContext;
use base_db::SourceDatabase;
use ide_db::assist_context::LocalAssists;
use ide_db::imports;
use lang::hir_db;
use lang::item_scope::NamedItemScope;
use lang::loc::SyntaxLocNodeExt;
use lang::nameres::fq_named_element::ItemFQNameOwner;
use lang::nameres::path_kind::path_kind;
use lang::nameres::scope::ScopeEntry;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::{FileRange, InFile};
use syntax::{AstNode, ast};

pub(crate) fn auto_import_fix(
    ctx: &DiagnosticsContext<'_>,
    path: InFile<ast::Path>,
    reference_range: FileRange,
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

    let path_scope = hir_db::item_scope(db, path.loc(file_id));
    let items_owner_scope = hir_db::item_scope(db, current_items_owner.loc(file_id));
    let add_test_only = path_scope == NamedItemScope::Test && items_owner_scope == NamedItemScope::Main;

    for import_candidate in import_candidates {
        add_autoimport_fix_for_import_candidate(
            db,
            &mut assists,
            import_candidate,
            &current_items_owner,
            reference_range,
            add_test_only,
        );
    }
    Some(assists)
}

fn add_autoimport_fix_for_import_candidate(
    db: &dyn SourceDatabase,
    assists: &mut LocalAssists,
    import_candidate: ScopeEntry,
    current_use_items_owner: &ast::AnyHasItems,
    reference_range: FileRange,
    add_test_only: bool,
) -> Option<()> {
    let candidate_named_element = import_candidate.cast_into::<ast::NamedElement>(db)?;
    let candidate_fq_name = candidate_named_element.fq_name(db)?;
    let fq_import_path = candidate_fq_name.fq_identifier_text();
    assists.add_fix_fallible(
        "add-import",
        format!("Add import for `{}`", fq_import_path),
        reference_range.range,
        imports::add_import_for_import_path(current_use_items_owner, fq_import_path, add_test_only),
    );
    Some(())
}
