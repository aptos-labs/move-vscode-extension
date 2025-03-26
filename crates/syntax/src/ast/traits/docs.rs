use crate::{ast, AstNode, AstToken};

pub trait DocCommentsOwner: AstNode {
    fn doc_comments(&self) -> Vec<ast::Comment> {
        self.syntax()
            .children_with_tokens()
            .into_iter()
            .filter_map(|it| {
                it.into_token()
                    .and_then(ast::Comment::cast)
                    .filter(|it| it.is_doc() && it.is_outer())
            })
            .collect()
    }
}
