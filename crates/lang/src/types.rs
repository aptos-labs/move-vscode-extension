// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

pub mod abilities;
pub mod has_type_params_ext;
pub mod inference;
pub mod lowering;
pub mod substitution;
pub mod ty;
pub mod ty_db;

pub mod render;

mod expectation;
pub mod fold;
mod patterns;
mod unification;
