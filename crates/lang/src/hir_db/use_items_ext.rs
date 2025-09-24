use crate::loc::{SyntaxLocFileExt, SyntaxLocInput};
use crate::nameres::node_ext::ModuleResolutionExt;
use crate::nameres::use_speck_entries::{UseItem, use_items_for_stmt};
use crate::node_ext::item::ModuleItemExt;
use base_db::SourceDatabase;
use syntax::ast::UseStmtsOwner;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::InFile;
use syntax::{AstNode, SyntaxNode, ast};

pub fn combined_use_items(
    db: &dyn SourceDatabase,
    use_stmts_owner: InFile<ast::AnyUseStmtsOwner>,
) -> Vec<UseItem> {
    use_items_from_self_and_siblings_tracked(db, SyntaxLocInput::new(db, use_stmts_owner.loc()))
}

pub fn use_items(
    db: &dyn SourceDatabase,
    use_stmts_owner: InFile<impl Into<ast::AnyUseStmtsOwner>>,
) -> Vec<UseItem> {
    use_items_tracked(
        db,
        SyntaxLocInput::new(db, use_stmts_owner.map(|it| it.into()).loc()),
    )
}

#[salsa_macros::tracked]
fn use_items_tracked<'db>(
    db: &'db dyn SourceDatabase,
    use_stmts_owner: SyntaxLocInput<'db>,
) -> Vec<UseItem> {
    use_stmts_owner
        .to_ast::<ast::AnyUseStmtsOwner>(db)
        .map(|use_stmts_owner| {
            let use_stmts = use_stmts_owner.flat_map(|it| it.use_stmts());
            use_stmts
                .into_iter()
                .flat_map(|stmt| use_items_for_stmt(db, stmt).unwrap_or_default())
                .collect()
        })
        .unwrap_or_default()
}

pub fn combined_use_stmts(
    db: &dyn SourceDatabase,
    use_stmts_owner: &InFile<ast::AnyUseStmtsOwner>,
) -> Vec<InFile<ast::UseStmt>> {
    let owner_with_siblings = use_stmts_owner_with_siblings(db, use_stmts_owner);
    owner_with_siblings
        .iter()
        .flat_map(|it| it.as_ref().flat_map(|owner| owner.use_stmts()))
        .collect()
}

pub fn reachable_paths(
    db: &dyn SourceDatabase,
    use_stmts_owner: &InFile<ast::AnyUseStmtsOwner>,
) -> Vec<InFile<ast::Path>> {
    let owner_with_siblings = use_stmts_owner_with_siblings(db, use_stmts_owner);
    let reachable_paths = owner_with_siblings.iter().flat_map(|it| {
        it.as_ref()
            .flat_map(|stmts_owner| descendant_paths(stmts_owner.syntax()))
    });
    reachable_paths.collect()
}

fn descendant_paths(node: &SyntaxNode) -> impl Iterator<Item = ast::Path> {
    node.descendants_of_type::<ast::Path>()
        .filter(|path| &path.base_path() == path)
        .filter(|path| !path.syntax().has_ancestor_strict::<ast::UseSpeck>())
}

fn use_stmts_owner_with_siblings(
    db: &dyn SourceDatabase,
    use_stmts_owner: &InFile<ast::AnyUseStmtsOwner>,
) -> Vec<InFile<ast::AnyUseStmtsOwner>> {
    let mut with_siblings = vec![use_stmts_owner.clone()];
    if let Some(module) = use_stmts_owner.cast_into_ref::<ast::Module>() {
        with_siblings.extend(
            module
                .related_module_specs(db)
                .into_iter()
                .map(|it| it.map_into()),
        );
    }
    if let Some(module_spec) = use_stmts_owner.cast_into_ref::<ast::ModuleSpec>() {
        if let Some(module) = module_spec.module(db) {
            with_siblings.push(module.clone().map_into());
        }
    }
    with_siblings
}

#[salsa_macros::tracked]
fn use_items_from_self_and_siblings_tracked<'db>(
    db: &'db dyn SourceDatabase,
    use_stmts_owner_loc: SyntaxLocInput<'db>,
) -> Vec<UseItem> {
    let owner_with_siblings = use_stmts_owner_loc
        .to_ast::<ast::AnyUseStmtsOwner>(db)
        .map(|use_stmts_owner| use_stmts_owner_with_siblings(db, &use_stmts_owner))
        .unwrap_or_default();
    owner_with_siblings
        .into_iter()
        .flat_map(|it| use_items(db, it))
        .collect()
}
