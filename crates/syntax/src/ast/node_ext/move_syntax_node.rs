use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::SyntaxKind::*;
use crate::{ast, AstNode, SyntaxNode};

pub trait MoveSyntaxNodeExt {
    fn containing_module(&self) -> Option<ast::Module>;
    fn containing_file(&self) -> Option<ast::SourceFile>;
    fn is_msl_only_item(&self) -> bool;
    fn is_msl_context(&self) -> bool;
    fn is<T: AstNode>(&self) -> bool;
    fn parent_is<T: AstNode>(&self) -> bool;
    fn cast<T: AstNode>(self) -> Option<T>;
}

impl MoveSyntaxNodeExt for SyntaxNode {
    fn containing_module(&self) -> Option<ast::Module> {
        self.ancestor_strict::<ast::Module>()
    }

    fn containing_file(&self) -> Option<ast::SourceFile> {
        self.ancestor_strict::<ast::SourceFile>()
    }

    fn is_msl_only_item(&self) -> bool {
        ast::AnyMslOnly::can_cast(self.kind())
    }

    fn is_msl_context(&self) -> bool {
        for ancestor in self.ancestors() {
            if ancestor.kind() == MODULE || ancestor.kind() == FUN || ancestor.kind() == STRUCT {
                return false;
            }
            if ancestor.is_msl_only_item() {
                return true;
            }
        }
        false
    }

    fn is<T: AstNode>(&self) -> bool {
        T::can_cast(self.kind())
    }

    fn parent_is<T: AstNode>(&self) -> bool {
        self.parent().is_some_and(|it| it.is::<T>())
    }

    fn cast<T: AstNode>(self) -> Option<T> {
        T::cast(self)
    }
}
