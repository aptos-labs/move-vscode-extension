// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

//! See docs for `SyntaxError`.

use rowan::{TextRange, TextSize};
use std::fmt;

/// Represents the result of unsuccessful tokenization, parsing
/// or tree validation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SyntaxError(String, TextRange);

impl SyntaxError {
    pub fn new(message: impl Into<String>, range: TextRange) -> Self {
        Self(message.into(), range)
    }

    pub fn new_at_offset(message: impl Into<String>, offset: TextSize) -> Self {
        Self(message.into(), TextRange::empty(offset))
    }

    pub fn range(&self) -> TextRange {
        self.1
    }

    pub fn with_range(mut self, range: TextRange) -> Self {
        self.1 = range;
        self
    }
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}
