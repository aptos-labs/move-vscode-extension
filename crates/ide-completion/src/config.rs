// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use ide_db::AllowSnippets;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompletionConfig {
    pub allow_snippets: Option<AllowSnippets>,
    pub enable_imports_on_the_fly: bool,
}
