pub mod nameres;
pub(crate) mod semantics;

pub mod builtin_files;
pub mod db;
mod item_scope;
pub mod loc;
pub mod node_ext;
pub mod types;

pub use crate::db::HirDatabase;
pub use semantics::{Semantics, SemanticsImpl};
