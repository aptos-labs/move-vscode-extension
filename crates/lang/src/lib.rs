// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

pub mod nameres;
pub(crate) mod semantics;

pub mod builtins_file;
pub mod hir_db;
mod item_scope;
pub mod loc;
pub mod node_ext;
pub mod types;

pub use semantics::{Semantics, SemanticsImpl};
