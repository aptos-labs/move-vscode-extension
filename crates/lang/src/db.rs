use crate::nameres::paths;
use crate::nameres::scope::ScopeEntry;
use base_db::{SourceDatabase, SourceRootDatabase, Upcast};
use parser::SyntaxKind;
use syntax::{ast, AstNode, SyntaxNode, TextRange, TextSize};
use vfs::FileId;
use crate::InFile;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SyntaxLoc {
    file_id: FileId,
    offset: TextSize,
    kind: SyntaxKind,
}

impl SyntaxLoc {
    pub fn from_syntax_node(node: InFile<&SyntaxNode>) -> Self {
        let InFile { file_id, value: syntax_node } = node;
        let range_start = syntax_node.text_range().start();
        let kind = syntax_node.kind();
        SyntaxLoc {
            file_id,
            offset: range_start,
            kind,
        }
    }
}

pub fn node_from_loc<T: ast::AstNode>(db: &dyn SourceDatabase, loc: SyntaxLoc) -> Option<T> {
    let file = db.parse(loc.file_id).tree();
    let token = file.syntax().token_at_offset(loc.offset).right_biased()?;
    for ancestor in token.parent_ancestors() {
        if T::can_cast(ancestor.kind()) {
            return T::cast(ancestor);
        }
    }
    None
}

#[ra_salsa::query_group(HirDatabaseStorage)]
pub trait HirDatabase: SourceDatabase + Upcast<dyn SourceDatabase> {
    // fn resolve_ast_path(&self, path_loc: SyntaxLoc) -> Vec<ScopeEntry>;
}

fn resolve_ast_path(db: &dyn HirDatabase, path_loc: SyntaxLoc) -> Vec<ScopeEntry> {
    let Some(path) = node_from_loc::<ast::Path>(db.upcast(), path_loc) else {
        return vec![];
    };
    paths::resolve_path(path)
}
