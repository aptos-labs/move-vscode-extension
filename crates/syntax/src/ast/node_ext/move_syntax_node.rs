use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{ast, SyntaxNode};

pub trait MoveSyntaxNodeExt {
    fn containing_module(&self) -> Option<ast::Module>;
    fn containing_file(&self) -> Option<ast::SourceFile>;
}

impl MoveSyntaxNodeExt for SyntaxNode {
    fn containing_module(&self) -> Option<ast::Module> {
        self.ancestor_strict::<ast::Module>()
    }

    fn containing_file(&self) -> Option<ast::SourceFile> {
        self.ancestor_strict::<ast::SourceFile>()
    }
}
