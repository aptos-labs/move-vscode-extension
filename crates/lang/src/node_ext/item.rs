use crate::HirDatabase;
use crate::nameres::ResolveReference;
use syntax::ast::ReferenceElement;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast, match_ast};

pub trait ModuleItemExt {
    fn module(&self, db: &dyn HirDatabase) -> Option<InFile<ast::Module>>;
}

impl ModuleItemExt for InFile<ast::ModuleSpec> {
    fn module(&self, db: &dyn HirDatabase) -> Option<InFile<ast::Module>> {
        let module_path = self.value.path()?;
        module_path
            .reference()
            .in_file(self.file_id)
            .resolve_no_inf(db)
            .and_then(|it| it.cast_into::<ast::Module>(db))
    }
}

impl ModuleItemExt for InFile<ast::AnyFun> {
    fn module(&self, db: &dyn HirDatabase) -> Option<InFile<ast::Module>> {
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
    fn module(&self, db: &dyn HirDatabase) -> Option<InFile<ast::Module>> {
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
    fn module(&self, db: &dyn HirDatabase) -> Option<InFile<ast::Module>> {
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
