// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use syntax::{AstToken, SyntaxKind, SyntaxToken, TokenAtOffset};

/// Picks the token with the highest rank returned by the passed in function.
pub fn pick_best_token(
    tokens: TokenAtOffset<SyntaxToken>,
    f: impl Fn(SyntaxKind) -> usize,
) -> Option<SyntaxToken> {
    tokens.max_by_key(move |t| f(t.kind()))
}

pub fn pick_token<T: AstToken>(mut tokens: TokenAtOffset<SyntaxToken>) -> Option<T> {
    tokens.find_map(T::cast)
}
