// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::nameres;
use base_db::SourceDatabase;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast, match_ast};

pub trait ModuleItemExt {
    fn module(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::Module>>;
}

impl ModuleItemExt for InFile<ast::ModuleSpec> {
    fn module(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::Module>> {
        let module_path = self.and_then_ref(|it| it.path())?;
        nameres::resolve_no_inf_cast::<ast::Module>(db, module_path)
    }
}

impl ModuleItemExt for InFile<ast::AnyFun> {
    fn module(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::Module>> {
        let module = self
            .clone()
            .and_then(|it| it.syntax().parent_of_type::<ast::Module>());
        if module.is_some() {
            return module;
        }
        let module_spec = self
            .clone()
            .and_then(|it| it.syntax().parent_of_type::<ast::ModuleSpec>());
        module_spec?.module(db)
    }
}

impl ModuleItemExt for InFile<ast::Schema> {
    fn module(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::Module>> {
        let module = self
            .clone()
            .and_then(|it| it.syntax().parent_of_type::<ast::Module>());
        if module.is_some() {
            return module;
        }
        let module_spec = self
            .clone()
            .and_then(|it| it.syntax().parent_of_type::<ast::ModuleSpec>());
        module_spec?.module(db)
    }
}

impl ModuleItemExt for InFile<ast::ItemSpec> {
    fn module(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::Module>> {
        let parent = self.value.syntax().parent()?;
        match_ast! {
            match parent {
                ast::Module(it) => Some(it.in_file(self.file_id)),
                ast::ModuleSpec(it) => {
                    it.in_file(self.file_id).module(db)
                },
                _ => None
            }
        }
    }
}
