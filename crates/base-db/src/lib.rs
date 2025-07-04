// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

#![allow(dead_code)]

pub mod change;
pub mod inputs;
pub mod package_root;
pub mod source_db;

pub use crate::source_db::SourceDatabase;
