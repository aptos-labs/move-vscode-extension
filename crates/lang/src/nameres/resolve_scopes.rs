use crate::nameres::node_ext::ModuleResolutionExt;
use crate::node_ext::item::ModuleItemExt;
use base_db::SourceDatabase;
use syntax::ast::HasItems;
use syntax::files::InFile;
use syntax::{AstNode, SyntaxNode, ast};

pub fn get_resolve_scopes(
    db: &dyn SourceDatabase,
    start_at: &InFile<SyntaxNode>,
) -> Vec<InFile<SyntaxNode>> {
    let (file_id, start_at) = start_at.as_ref().unpack();

    // NOTE: caching top-down file traverse making perf worse
    let mut scopes = start_at
        .ancestors()
        // skip the current node
        .skip(1)
        .map(|scope| InFile::new(file_id, scope))
        .collect::<Vec<_>>();

    // drop SOURCE_FILE
    scopes.pop();

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
pub(crate) fn module_inner_spec_scopes(item_container: &InFile<ast::Module>) -> Vec<InFile<SyntaxNode>> {
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
