use crate::nameres::node_ext::ModuleResolutionExt;
use crate::node_ext::item::ModuleItemExt;
use base_db::SourceDatabase;
use std::collections::HashSet;
use std::sync::LazyLock;
use syntax::SyntaxKind::*;
use syntax::ast::HasItems;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, SyntaxKind, SyntaxNode, ast};

static VALID_RESOLVE_SCOPES: LazyLock<HashSet<SyntaxKind>> = LazyLock::new(|| {
    vec![
        MODULE,
        MODULE_SPEC,
        SCRIPT,
        ITEM_SPEC,
        FUN,
        SPEC_FUN,
        SPEC_INLINE_FUN,
        LAMBDA_EXPR,
        SCHEMA,
        FOR_EXPR,
        FORALL_EXPR,
        EXISTS_EXPR,
        CHOOSE_EXPR,
        AXIOM_STMT,
        INVARIANT_STMT,
        APPLY_SCHEMA,
        MATCH_ARM,
        BLOCK_EXPR,
        GLOBAL_VARIABLE_DECL,
        ENUM,
        STRUCT,
        SPEC_BLOCK_EXPR,
    ]
    .into_iter()
    .collect()
});

// NOTE: caching top-down file traverse making perf worse
pub(crate) fn get_resolve_scopes(
    db: &dyn SourceDatabase,
    start_at: &InFile<SyntaxNode>,
) -> Vec<InFile<SyntaxNode>> {
    let (file_id, start_at) = start_at.as_ref().unpack();

    let mut scopes = Vec::with_capacity(8);
    let mut opt_scope = start_at.parent();

    while let Some(scope) = opt_scope.take() {
        opt_scope = scope.parent();
        if VALID_RESOLVE_SCOPES.contains(&scope.kind()) {
            scopes.push(scope.in_file(file_id));
        }
    }

    let last_scope = scopes.last().cloned();
    if let Some(last_scope) = last_scope {
        if let Some(module) = last_scope.syntax_cast::<ast::Module>() {
            scopes.extend(module_inner_spec_scopes(&module));
            for related_module_spec in module.related_module_specs(db) {
                scopes.push(related_module_spec.syntax());
            }
        }
        if let Some(module_spec) = last_scope.syntax_cast::<ast::ModuleSpec>() {
            let start_at_offset = start_at.text_range().start();
            // skip if we're resolving module path for the module spec
            if module_spec
                .value
                .path()
                .is_none_or(|it| !it.syntax().text_range().contains(start_at_offset))
            {
                if let Some(module) = module_spec.module(db) {
                    scopes.push(module.as_ref().map(|it| it.syntax().clone()));
                    scopes.extend(module_inner_spec_scopes(&module));
                }
            }
        }
    }

    scopes
}

// all `spec module {}` in item container
fn module_inner_spec_scopes(item_container: &InFile<ast::Module>) -> Vec<InFile<SyntaxNode>> {
    let (file_id, module) = item_container.as_ref().unpack();
    let mut inner_scopes = vec![];
    for module_item_spec in module.module_item_specs() {
        if let Some(module_item_spec_block) = module_item_spec.spec_block() {
            let scope = module_item_spec_block.syntax().to_owned();
            inner_scopes.push(InFile::new(file_id, scope))
        }
    }
    inner_scopes
}
