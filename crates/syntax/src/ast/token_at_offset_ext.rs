use crate::SyntaxKind::WHITESPACE;
use crate::SyntaxToken;
use crate::ast::support::token;
use rowan::TokenAtOffset;

pub trait TokenAtOffsetExt {
    fn token_at_offset(self) -> TokenAtOffset<SyntaxToken>;

    /// If between whitespace and token, prefer non-whitespace.
    fn prefer_no_trivia(self) -> Option<SyntaxToken>
    where
        Self: Sized,
    {
        match self.token_at_offset() {
            TokenAtOffset::None => None,
            TokenAtOffset::Single(token) => Some(token),
            TokenAtOffset::Between(left, right) => {
                if !left.kind().is_trivia() {
                    return Some(left);
                }
                if !right.kind().is_trivia() {
                    return Some(right);
                }
                None
            }
        }
    }
}

impl TokenAtOffsetExt for TokenAtOffset<SyntaxToken> {
    fn token_at_offset(self) -> TokenAtOffset<SyntaxToken> {
        self
    }
}
