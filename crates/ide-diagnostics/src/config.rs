// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct DiagnosticsConfig {
    /// Whether native diagnostics are enabled.
    pub enabled: bool,
    pub disabled: HashSet<String>,
    pub needs_type_annotation: bool,
    pub assists_only: bool,
}

impl DiagnosticsConfig {
    pub fn test_sample() -> Self {
        Self {
            enabled: true,
            disabled: Default::default(),
            needs_type_annotation: true,
            assists_only: false,
        }
    }

    pub fn for_assists(mut self) -> Self {
        self.assists_only = true;
        self
    }
}
