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

mod command;
pub mod diagnostics;
mod flycheck;
pub mod lsp;
mod project_folders;
pub mod toolchain;
pub mod tracing;

pub use config::{config_change::ConfigChange, Config, ConfigErrors};
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

fn main() {}
