use base_db::{SourceDatabase, Upcast};

#[ra_salsa::query_group(HirDatabaseStorage)]
pub trait HirDatabase: SourceDatabase + Upcast<dyn SourceDatabase> {}
