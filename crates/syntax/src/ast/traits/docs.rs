use crate::ast::HasAttrs;
use crate::{ast, AstNode, AstToken, SyntaxElementChildren};

pub trait HasDocComments: AstNode {
    fn doc_comments(&self) -> DocCommentIter {
        DocCommentIter {
            iter: self.syntax().children_with_tokens(),
        }
    }
}

pub struct DocCommentIter {
    iter: SyntaxElementChildren,
}

impl DocCommentIter {
    pub fn from_syntax_node(syntax_node: &ast::SyntaxNode) -> DocCommentIter {
        DocCommentIter {
            iter: syntax_node.children_with_tokens(),
        }
    }
}

impl Iterator for DocCommentIter {
    type Item = ast::Comment;
    fn next(&mut self) -> Option<ast::Comment> {
        self.iter.by_ref().find_map(|el| {
            el.into_token()
                .and_then(ast::Comment::cast)
                .filter(ast::Comment::is_doc)
        })
    }
}
