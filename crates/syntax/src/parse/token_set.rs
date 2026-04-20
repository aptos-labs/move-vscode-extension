// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::{SyntaxKind, T};
use std::fmt;
use std::ops::{Add, BitOr};

/// A bit-set of `SyntaxKind`s
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct TokenSet(pub u128);

impl fmt::Debug for TokenSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.kinds().iter()).finish()
    }
}

impl From<SyntaxKind> for TokenSet {
    fn from(value: SyntaxKind) -> Self {
        TokenSet::new(&[value])
    }
}

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
        TokenSet::EMPTY
    );
    ($($x:expr),+ $(,)?) => (
        TokenSet::new(&[$($x),+])
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
        TokenSet(self.0 & !other.0)
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub(crate) const fn contains(&self, kind: SyntaxKind) -> bool {
        self.0 & mask(kind) != 0
    }

    pub(crate) fn kinds(&self) -> Vec<SyntaxKind> {
        (0..128u16)
            .map(|i| unsafe { std::mem::transmute(i) })
            .filter(|k| self.contains(*k))
            .collect()
    }
}

pub const fn mask(kind: SyntaxKind) -> u128 {
    1u128 << (kind as usize)
}
