//! A bit-set of `SyntaxKind`s.

use crate::SyntaxKind;
use std::ops::{Add, BitOr};

/// A bit-set of `SyntaxKind`s
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TokenSet(pub u128);

impl Add<TokenSet> for TokenSet {
    type Output = TokenSet;
    fn add(self, rhs: TokenSet) -> Self::Output {
        self.union(rhs)
    }
}

impl BitOr<SyntaxKind> for TokenSet {
    type Output = TokenSet;
    fn bitor(self, rhs: SyntaxKind) -> Self::Output {
        self.union(TokenSet::new(&[rhs]))
    }
}

impl BitOr<SyntaxKind> for SyntaxKind {
    type Output = TokenSet;
    fn bitor(self, rhs: SyntaxKind) -> Self::Output {
        TokenSet::new(&[self, rhs])
    }
}

impl BitOr<TokenSet> for SyntaxKind {
    type Output = TokenSet;
    fn bitor(self, rhs: TokenSet) -> Self::Output {
        TokenSet::new(&[self]) + rhs
    }
}

#[macro_export]
macro_rules! ts {
    () => (
        crate::parse::token_set::TokenSet::EMPTY
    );
    ($($x:expr),+ $(,)?) => (
        crate::parse::token_set::TokenSet::new(&[$($x),+])
    );
}

impl TokenSet {
    pub(crate) const EMPTY: TokenSet = TokenSet(0);

    pub(crate) const fn new(kinds: &[SyntaxKind]) -> TokenSet {
        let mut res = 0u128;
        let mut i = 0;
        while i < kinds.len() {
            res |= mask(kinds[i]);
            i += 1;
        }
        TokenSet(res)
    }

    pub(crate) const fn union(self, other: TokenSet) -> TokenSet {
        TokenSet(self.0 | other.0)
    }

    pub(crate) const fn sub(self, other: TokenSet) -> TokenSet {
        TokenSet(self.0 ^ other.0)
    }

    pub(crate) const fn contains(&self, kind: SyntaxKind) -> bool {
        self.0 & mask(kind) != 0
    }
}

const fn mask(kind: SyntaxKind) -> u128 {
    1u128 << (kind as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::T;

    #[test]
    fn test_token_sets() {
        assert_eq!(ts!(T![,], T![;]), TokenSet::new(&[T![,], T![;]]));

        assert_eq!(ts!(T![,], T![;]) | T![:], ts!(T![,], T![;], T![:]));
        assert_eq!(ts!(T![,], T![;]) + ts!(T![:]), ts!(T![,], T![;], T![:]));
    }

    #[test]
    fn test_sub() {
        assert_eq!(ts!(T![,], T![')']).sub(ts!(T![,])), ts!(T![')']));
    }
}
