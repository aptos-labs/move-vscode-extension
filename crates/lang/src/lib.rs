pub mod nameres;
pub(crate) mod semantics;

pub mod builtin_files;
pub mod db;
pub mod loc;
pub mod node_ext;
pub mod types;
mod item_scope;

pub use crate::db::HirDatabase;
pub use semantics::{Semantics, SemanticsImpl};
