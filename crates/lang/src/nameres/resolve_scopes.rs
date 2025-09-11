use crate::nameres::node_ext::ModuleResolutionExt;
use crate::node_ext::item::ModuleItemExt;
use base_db::SourceDatabase;
use std::fmt;
use std::fmt::Formatter;
use syntax::SyntaxKind::MODULE_SPEC;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, SyntaxKind, SyntaxNode, ast};

pub fn get_resolve_scopes(db: &dyn SourceDatabase, start_at: &InFile<SyntaxNode>) -> Vec<ResolveScope> {
    let (file_id, start_at) = start_at.as_ref().unpack();

    let mut scopes = vec![];
    let mut opt_scope = start_at.parent();

    while let Some(ref scope) = opt_scope {
        scopes.push(ResolveScope {
            scope: InFile::new(file_id, scope.clone()),
        });

        if scope.kind() == SyntaxKind::MODULE {
            let module = ast::Module::cast(scope.clone()).unwrap().in_file(file_id);
            scopes.extend(module_inner_spec_scopes(module.clone()));

            for related_module_spec in module.related_module_specs(db) {
                scopes.push(ResolveScope {
                    scope: related_module_spec.syntax(),
                });
            }
            break;
        }

        if scope.kind() == MODULE_SPEC {
            let module_spec = scope.clone().cast::<ast::ModuleSpec>().unwrap();
            if module_spec
                .path()
                .is_none_or(|it| it.syntax().text_range().contains(start_at.text_range().start()))
            {
                // skip if we're resolving module path for the module spec
                break;
            }
            if let Some(module) = module_spec.clone().in_file(file_id).module(db) {
                scopes.push(ResolveScope {
                    scope: module.clone().map(|it| it.syntax().clone()),
                });
                scopes.extend(module_inner_spec_scopes(module));
            }
            break;
        }

        let parent_scope = scope.parent();
        opt_scope = parent_scope;
    }

    scopes
}

pub struct ResolveScope {
    scope: InFile<SyntaxNode>,
}

impl ResolveScope {
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
fn module_inner_spec_scopes(item_container: InFile<impl ast::HasItems>) -> Vec<ResolveScope> {
    let (file_id, module) = item_container.unpack();
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
