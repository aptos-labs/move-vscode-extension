// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::SyntaxKind;
use std::fmt;
use std::ops::{Add, BitOr};

/// Number of u128 chunks. 3 × 128 = 384 bits, enough for all ~305 SyntaxKind variants.
const CHUNKS: usize = 3;

/// A bit-set of `SyntaxKind`s, stored as a fixed-size array of u128 chunks.
/// Each SyntaxKind's discriminant maps to one bit: chunk index = discriminant / 128,
/// bit position within the chunk = discriminant % 128.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct TokenSet([u128; CHUNKS]);

impl fmt::Debug for TokenSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Entry(u16);

        impl fmt::Debug for Entry {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if self.0 <= SyntaxKind::__LAST as u16 {
                    SyntaxKind::from(self.0).fmt(f)
                } else {
                    self.0.fmt(f)
                }
            }
        }

        f.debug_set().entries(self.indices().map(Entry)).finish()
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
    pub const EMPTY: TokenSet = TokenSet([0; CHUNKS]);

    /// Set the bit for each kind: OR the kind's mask into the appropriate chunk.
    pub const fn new(kinds: &[SyntaxKind]) -> TokenSet {
        let mut res = [0u128; CHUNKS];
        let mut i = 0;
        while i < kinds.len() {
            let (chunk, mask) = chunk_and_mask(kinds[i]);
            res[chunk] |= mask;
            i += 1;
        }
        TokenSet(res)
    }

    /// Bitwise OR of each chunk — result contains kinds present in either set.
    pub const fn union(self, other: TokenSet) -> TokenSet {
        let mut res = [0u128; CHUNKS];
        let mut i = 0;
        while i < CHUNKS {
            res[i] = self.0[i] | other.0[i];
            i += 1;
        }
        TokenSet(res)
    }

    /// `self & !other` per chunk — result contains kinds in self but not in other.
    pub const fn sub(self, other: TokenSet) -> TokenSet {
        let mut res = [0u128; CHUNKS];
        let mut i = 0;
        while i < CHUNKS {
            res[i] = self.0[i] & !other.0[i];
            i += 1;
        }
        TokenSet(res)
    }

    /// All chunks are zero — no bits set.
    pub const fn is_empty(&self) -> bool {
        let mut i = 0;
        while i < CHUNKS {
            if self.0[i] != 0 {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Test the single bit for this kind: AND the chunk with the kind's mask.
    pub const fn contains(&self, kind: SyntaxKind) -> bool {
        let (chunk, mask) = chunk_and_mask(kind);
        self.0[chunk] & mask != 0
    }

    /// Enumerate all set bits back into SyntaxKind values.
    pub fn kinds(&self) -> Vec<SyntaxKind> {
        (0..=(SyntaxKind::__LAST as u16))
            .map(SyntaxKind::from)
            .filter(|k| self.contains(*k))
            .collect()
    }

    fn indices(&self) -> impl Iterator<Item = u16> + '_ {
        (0..(128 * CHUNKS) as u16).filter(|idx| {
            let chunk = *idx as usize / 128;
            let mask = 1u128 << (*idx as usize % 128);
            self.0[chunk] & mask != 0
        })
    }
}

/// Maps a SyntaxKind to its chunk index and single-bit mask within that chunk.
const fn chunk_and_mask(kind: SyntaxKind) -> (usize, u128) {
    let idx = kind as usize;
    (idx / 128, 1u128 << (idx % 128))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_empty_token_set() {
        assert_eq!(format!("{:?}", TokenSet::EMPTY), "{}");
    }

    #[test]
    fn debug_token_set_uses_syntax_kind_names() {
        let token_set = TokenSet::new(&[SyntaxKind::IDENT, SyntaxKind::SEMICOLON]);

        assert_eq!(format!("{token_set:?}"), "{SEMICOLON, IDENT}");
    }

    #[test]
    fn debug_token_set_uses_integer_for_unknown_bits() {
        let unknown_idx = SyntaxKind::__LAST as usize + 1;
        assert!(unknown_idx < 128 * CHUNKS);

        let mut chunks = [0u128; CHUNKS];
        chunks[unknown_idx / 128] = 1u128 << (unknown_idx % 128);
        let token_set = TokenSet(chunks);

        assert_eq!(format!("{token_set:?}"), format!("{{{unknown_idx}}}"));
    }
}
