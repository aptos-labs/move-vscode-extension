#![allow(dead_code)]

pub mod apply_change;
pub mod assist_config;
pub mod assists;
pub mod defs;
pub mod helpers;
pub mod label;
pub mod source_change;
mod syntax_helpers;
pub mod text_edit;
pub mod new_root_db;

use base_db::SourceDatabase;
use lang::db::HirDatabase;
use line_index::LineIndex;
use std::fmt;
use std::mem::ManuallyDrop;
use std::sync::Arc;
use syntax::{SyntaxKind, SyntaxKind::*};
use vfs::FileId;

#[ra_salsa::database(
    base_db::SourceDatabaseStorage,
    lang::db::HirDatabaseStorage,
    LineIndexDatabaseStorage
)]
pub struct RootDatabase {
    // We use `ManuallyDrop` here because every codegen unit that contains a
    // `&RootDatabase -> &dyn OtherDatabase` cast will instantiate its drop glue in the vtable,
    // which duplicates `Weak::drop` and `Arc::drop` tens of thousands of times, which makes
    // compile times of all `ide_*` and downstream crates suffer greatly.
    storage: ManuallyDrop<ra_salsa::Storage<RootDatabase>>,
}

impl Drop for RootDatabase {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.storage) };
    }
}

impl fmt::Debug for RootDatabase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RootDatabase").finish()
    }
}

impl ra_salsa::Database for RootDatabase {}

impl Default for RootDatabase {
    fn default() -> RootDatabase {
        RootDatabase::new(/*None*/)
    }
}

impl RootDatabase {
    pub fn new() -> RootDatabase {
        let mut db = RootDatabase {
            storage: ManuallyDrop::new(ra_salsa::Storage::default()),
        };

        db.set_builtins_file_id(None);

        // db.set_local_roots_with_durability(Default::default(), Durability::HIGH);
        // db.set_library_roots_with_durability(Default::default(), Durability::HIGH);
        // db.setup_syntax_context_root();
        db
    }
}

impl ra_salsa::ParallelDatabase for RootDatabase {
    fn snapshot(&self) -> ra_salsa::Snapshot<RootDatabase> {
        ra_salsa::Snapshot::new(RootDatabase {
            storage: ManuallyDrop::new(self.storage.snapshot()),
        })
    }
}

#[ra_salsa::query_group(LineIndexDatabaseStorage)]
pub trait LineIndexDatabase: SourceDatabase {
    fn line_index(&self, file_id: FileId) -> Arc<LineIndex>;
}

fn line_index(db: &dyn LineIndexDatabase, file_id: FileId) -> Arc<LineIndex> {
    let text = db.file_text(file_id);
    Arc::new(LineIndex::new(&text))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolKind {
    Attribute,
    Module,
    Const,
    Function,
    Method,
    Struct,
    Field,
    Enum,
    EnumVariant,
    TypeParam,
    ValueParam,
    Label,
    Local,
    // SpecFunction,
    // SpecInlineFunction,
    // Schema,
    // ModuleSpec,
    // ItemSpec,
}

pub fn ast_kind_to_symbol_kind(kind: SyntaxKind) -> Option<SymbolKind> {
    match kind {
        MODULE => Some(SymbolKind::Module),

        FUN | SPEC_FUN | SPEC_INLINE_FUN => Some(SymbolKind::Function),

        CONST => Some(SymbolKind::Const),
        STRUCT => Some(SymbolKind::Struct),
        ENUM => Some(SymbolKind::Enum),

        TYPE_PARAM => Some(SymbolKind::TypeParam),
        IDENT_PAT => Some(SymbolKind::Local),
        VARIANT => Some(SymbolKind::EnumVariant),

        NAMED_FIELD => Some(SymbolKind::Field),

        // todo
        SCHEMA => Some(SymbolKind::Struct),

        _ => {
            tracing::error!("unhandled ast kind {:?}", kind);
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnippetCap {
    _private: (),
}

impl SnippetCap {
    pub const fn new(allow_snippets: bool) -> Option<SnippetCap> {
        if allow_snippets {
            Some(SnippetCap { _private: () })
        } else {
            None
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    WeakWarning,
    Allow,
}

impl Severity {
    pub fn from_test_ident(ident: &str) -> Severity {
        let expected_severity = match ident {
            "err:" => Severity::Error,
            "warn:" => Severity::Warning,
            "weak:" => Severity::WeakWarning,
            "allow:" => Severity::Allow,
            _ => unreachable!("unknown severity {:?}", ident),
        };
        expected_severity
    }

    pub fn to_test_ident(&self) -> &str {
        match self {
            Severity::Error => "err:",
            Severity::Warning => "warn",
            Severity::WeakWarning => "weak:",
            Severity::Allow => "allow:",
        }
    }
}
