// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::UseSpeck {
    pub fn path_name(&self) -> Option<String> {
        self.path()?.reference_name()
    }

    pub fn parent_use_group(&self) -> Option<ast::UseGroup> {
        self.syntax.parent_of_type()
    }

    pub fn use_stmt(&self) -> Option<ast::UseStmt> {
        self.syntax.parent_of_type()
    }

    pub fn is_group_self(&self) -> bool {
        self.parent_use_group().is_some() && self.is_self_name()
    }

    pub fn is_root_self(&self) -> bool {
        self.parent_use_group().is_none()
            && self.path().is_some_and(|it| it.qualifier().is_some())
            && self.is_self_name()
    }

    fn is_self_name(&self) -> bool {
        self.path_name().is_some_and(|it| it == "Self")
    }

    pub fn kind(&self) -> Option<UseSpeckKind> {
        let root_path = self.path()?;
        if let Some(use_group) = self.use_group() {
            return Some(UseSpeckKind::Group {
                module_path: root_path,
                name_refs: use_group.name_refs_with_aliases(),
            });
        }
        if self.parent_use_group().is_some() {
            if let Some(path) = self.path()
                && path.qualifier().is_none()
            {
                let use_alias = self.use_alias();
                return Some(UseSpeckKind::GroupNameRef {
                    name_ref: path.segment().and_then(|it| it.name_ref()),
                    use_alias,
                });
            }
        }
        let qualifier = root_path.qualifier()?;
        match qualifier.qualifier() {
            None => {
                // two element path, return original path
                Some(UseSpeckKind::Module {
                    path: root_path,
                    alias: self.use_alias(),
                })
            }
            Some(_) => {
                // three element, return qualifier
                Some(UseSpeckKind::Item {
                    module_path: qualifier,
                    item_name_ref: root_path.segment().and_then(|it| it.name_ref()),
                    alias: self.use_alias(),
                })
            }
        }
    }
}

pub enum UseSpeckKind {
    Module {
        path: ast::Path,
        alias: Option<ast::UseAlias>,
    },
    Item {
        module_path: ast::Path,
        item_name_ref: Option<ast::NameRef>,
        alias: Option<ast::UseAlias>,
    },
    Group {
        module_path: ast::Path,
        name_refs: Vec<(ast::NameRef, Option<ast::UseAlias>)>,
    },
    GroupNameRef {
        name_ref: Option<ast::NameRef>,
        use_alias: Option<ast::UseAlias>,
    },
}

impl UseSpeckKind {
    pub fn into_group_name_ref(self) -> Option<(ast::NameRef, Option<ast::UseAlias>)> {
        match self {
            UseSpeckKind::GroupNameRef { name_ref, use_alias } => name_ref.map(|it| (it, use_alias)),
            _ => None,
        }
    }
}
