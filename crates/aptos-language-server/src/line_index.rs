// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

//! Enhances `ide::LineIndex` with additional info required to convert offsets
//! into lsp positions.
//!
//! We maintain invariant that all internal strings use `\n` as line separator.
//! This module does line ending conversion and detection (so that we can
//! convert back to `\r\n` on the way out).

use ide_db::line_endings::LineEndings;
use std::sync::Arc;

#[derive(Clone, Copy)]
pub enum PositionEncoding {
    Utf8,
    Wide(line_index::WideEncoding),
}

pub(crate) struct LineIndex {
    pub(crate) index: Arc<line_index::LineIndex>,
    pub(crate) endings: LineEndings,
    pub(crate) encoding: PositionEncoding,
}
