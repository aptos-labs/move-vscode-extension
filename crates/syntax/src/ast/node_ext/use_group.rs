// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::UseGroup {
    pub fn parent_use_speck(&self) -> ast::UseSpeck {
        self.syntax
            .parent_of_type::<ast::UseSpeck>()
            .expect("always exists")
    }

    pub fn use_stmt(&self) -> Option<ast::UseStmt> {
        self.parent_use_speck().use_stmt()
    }

    pub fn use_specks_with_aliases(&self) -> Vec<(ast::UseSpeck, Option<ast::UseAlias>)> {
        self.use_specks().map(|it| (it.clone(), it.use_alias())).collect()
    }

    pub fn name_refs_with_aliases(&self) -> Vec<(ast::NameRef, Option<ast::UseAlias>)> {
        let mut name_refs_with_aliases = vec![];
        for (use_speck, use_alias) in self.use_specks_with_aliases() {
            if let Some((name_ref, use_alias)) = use_speck.kind().and_then(|it| it.into_group_name_ref())
            {
                name_refs_with_aliases.push((name_ref, use_alias));
            }
        }
        name_refs_with_aliases
    }
}
