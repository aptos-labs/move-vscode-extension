use crate::SyntaxKind::WHITESPACE;
use crate::ast::UseStmtsOwner;
use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::ast::syntax_factory::SyntaxFactory;
use crate::syntax_editor::{Element, Position, SyntaxEditor};
use crate::{AstNode, SyntaxNodeOrToken, ast, match_ast};

impl ast::AnyHasItems {
    pub fn pos_after_last_use_stmt(&self) -> Option<(Position, bool)> {
        let anchor = self
            .use_stmts()
            .last()
            .map(|it| it.syntax().syntax_element())
            .or_else(|| {
                match_ast! {
                    match (self.syntax) {
                        ast::Module(it) => it.l_curly_token(),
                        ast::Script(it) => it.l_curly_token(),
                        ast::ModuleSpec(it) => it.l_curly_token(),
                        _ => None,
                    }
                }
                .map(|it| it.syntax_element())
            });
        let mut has_extra_newline_at_the_end = false;
        if let Some(anchor) = &anchor
            && let Some(next_token) = anchor.next_token()
            && next_token.kind().is_whitespace()
            && next_token.text().chars().filter(|it| *it == '\n').count() > 1
        {
            has_extra_newline_at_the_end = true;
        }
        anchor.map(|it| (Position::after(it), !has_extra_newline_at_the_end))
    }
}
