// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

#![allow(dead_code)]

mod config;
mod global_state;
mod handlers;
mod line_index;
mod main_loop;
mod mem_docs;
mod op_queue;
mod reload;
mod task_pool;
mod version;

pub mod cli;
mod command;
pub mod compiler_diagnostic;
pub mod diagnostics;
mod file_changes;
pub mod lsp;
mod movefmt;
pub mod toolchain;
pub mod tracing;

pub use config::{Config, config_change::ConfigChange, validation::ConfigErrors};
pub use lsp::capabilities::server_capabilities;
pub use lsp::ext as lsp_ext;
pub use main_loop::main_loop;
pub use version::version;

use serde::de::DeserializeOwned;

pub fn from_json<T: DeserializeOwned>(
    what: &'static str,
    json: &serde_json::Value,
) -> anyhow::Result<T> {
    serde_json::from_value(json.clone())
        .map_err(|e| anyhow::format_err!("Failed to deserialize {what}: {e}; {json}"))
}

#[doc(hidden)]
macro_rules! try_default_ {
    ($it:expr $(,)?) => {
        match $it {
            Some(it) => it,
            None => return Ok(Default::default()),
        }
    };
}
pub(crate) use try_default_ as try_default;
