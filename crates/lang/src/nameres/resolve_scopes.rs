use crate::nameres::node_ext::ModuleResolutionExt;
use crate::node_ext::item::ModuleItemExt;
use base_db::SourceDatabase;
use std::fmt;
use std::fmt::Formatter;
use syntax::ast::HasItems;
use syntax::files::InFile;
use syntax::{AstNode, SyntaxNode, ast};
use vfs::FileId;

pub fn get_resolve_scopes(db: &dyn SourceDatabase, start_at: &InFile<SyntaxNode>) -> Vec<ResolveScope> {
    let (file_id, start_at) = start_at.as_ref().unpack();

    // NOTE: caching top-down file traverse making perf worse
    let mut scopes = start_at
        .ancestors()
        // skip the current node
        .skip(1)
        .map(|scope| ResolveScope::new(file_id, scope))
        .collect::<Vec<_>>();

    // drop SOURCE_FILE
    scopes.pop();

    let last_scope = scopes.last().map(|it| it.scope()).cloned();
    if let Some(last_scope) = last_scope {
        if let Some(module) = last_scope.syntax_cast::<ast::Module>() {
            scopes.extend(module_inner_spec_scopes(&module));
            for related_module_spec in module.related_module_specs(db) {
                scopes.push(ResolveScope {
                    scope: related_module_spec.syntax(),
                });
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
                    scopes.push(ResolveScope {
                        scope: module.clone().map(|it| it.syntax().clone()),
                    });
                    scopes.extend(module_inner_spec_scopes(&module));
                }
            }
        }
    }

    scopes
}

#[derive(Clone)]
pub struct ResolveScope {
    pub scope: InFile<SyntaxNode>,
}

impl ResolveScope {
    pub fn new(file_id: FileId, scope: SyntaxNode) -> Self {
        ResolveScope {
            scope: InFile::new(file_id, scope),
        }
    }
    pub fn scope(&self) -> &InFile<SyntaxNode> {
        &self.scope
    }
}

impl fmt::Debug for ResolveScope {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_set().entry(&self.scope.value.kind()).finish()
    }
}

// all `spec module {}` in item container
pub(crate) fn module_inner_spec_scopes(item_container: &InFile<ast::Module>) -> Vec<ResolveScope> {
    let (file_id, module) = item_container.as_ref().unpack();
    let mut inner_scopes = vec![];
    for module_item_spec in module.module_item_specs() {
        if let Some(module_item_spec_block) = module_item_spec.spec_block() {
            let scope = module_item_spec_block.syntax().to_owned();
            inner_scopes.push(ResolveScope {
                scope: InFile::new(file_id, scope),
            })
        }
    }
    inner_scopes
}
