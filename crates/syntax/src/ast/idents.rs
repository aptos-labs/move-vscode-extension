// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use std::cell::OnceCell;
use std::sync::LazyLock;
use stdx::itertools::Itertools;

pub const INTEGER_IDENTS: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128", "i256", "u8", "u16", "u32", "u64", "u128", "u256", "num",
];

pub static PRIMITIVE_TYPES: LazyLock<Vec<&str>> =
    LazyLock::new(|| [&["bool", "address", "signer", "vector", "bv"], INTEGER_IDENTS].concat());
