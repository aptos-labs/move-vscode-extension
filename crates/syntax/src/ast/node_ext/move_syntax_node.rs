use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::SyntaxKind::*;
use crate::{ast, AstNode, SyntaxNode};

pub trait MoveSyntaxElementExt {
    fn node(&self) -> &SyntaxNode;

    fn containing_module(&self) -> Option<ast::Module> {
        self.node().ancestor_strict::<ast::Module>()
    }

    fn containing_file(&self) -> Option<ast::SourceFile> {
        self.node().ancestor_strict::<ast::SourceFile>()
    }

    fn is<T: AstNode>(&self) -> bool {
        T::can_cast(self.node().kind())
    }

    fn parent_is<T: AstNode>(&self) -> bool {
        self.node().parent().is_some_and(|it| it.is::<T>())
    }

    fn is_msl_only_item(&self) -> bool {
        self.is::<ast::AnyMslOnly>()
    }

    fn is_msl_only_scope(&self) -> bool {
        matches!(
            self.node().kind(),
            SPEC_FUN | SPEC_INLINE_FUN | ITEM_SPEC | SPEC_BLOCK_EXPR | SCHEMA
        )
    }

    fn is_msl_context(&self) -> bool {
        for ancestor in self.node().ancestors() {
            if matches!(ancestor.kind(), MODULE | FUN | STRUCT | ENUM) {
                return false;
            }
            if ancestor.is_msl_only_item() {
                return true;
            }
        }
        false
    }

    fn cast<T: AstNode>(&self) -> Option<T> {
        T::cast(self.node().clone())
    }

    fn inference_ctx_owner(&self) -> Option<ast::InferenceCtxOwner> {
        self.node().ancestor_or_self::<ast::InferenceCtxOwner>()
    }
}

impl MoveSyntaxElementExt for SyntaxNode {
    fn node(&self) -> &SyntaxNode {
        self
    }
}
