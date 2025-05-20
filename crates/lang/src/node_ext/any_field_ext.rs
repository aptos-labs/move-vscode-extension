use crate::loc::SyntaxLocFileExt;
use crate::nameres::namespaces::Ns;
use crate::nameres::scope::{ScopeEntry, ScopeEntryExt};
use syntax::ast;
use syntax::files::InFileExt;
use vfs::FileId;

pub(crate) fn to_scope_entry(name: String, file_id: FileId, field: ast::AnyField) -> Option<ScopeEntry> {
    match field {
        ast::AnyField::NamedField(named_field) => named_field.in_file(file_id).to_entry(),
        ast::AnyField::TupleField(tuple_field) => Some(ScopeEntry {
            name,
            node_loc: tuple_field.in_file(file_id).loc(),
            ns: Ns::NAME,
            scope_adjustment: None,
        }),
    }
}
