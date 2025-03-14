use crate::InFile;
use base_db::SourceRootDatabase;
use parser::SyntaxKind;
use std::fmt;
use std::fmt::Formatter;
use syntax::algo::find_node_at_offset;
use syntax::{AstNode, TextSize};
use vfs::FileId;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct SyntaxLoc {
    file_id: FileId,
    offset: TextSize,
    kind: SyntaxKind,
}

impl SyntaxLoc {
    pub fn from_ast_node<T: AstNode>(node: InFile<T>) -> Self {
        let InFile {
            file_id,
            value: syntax_node,
        } = node;
        let range_start = syntax_node.syntax().text_range().end();
        let kind = syntax_node.syntax().kind();
        SyntaxLoc {
            file_id,
            offset: range_start,
            kind,
        }
    }

    pub fn cast<T: AstNode>(self, db: &dyn SourceRootDatabase) -> Option<InFile<T>> {
        let file = db.parse(self.file_id).tree();
        let node = find_node_at_offset::<T>(file.syntax(), self.offset)?;
        Some(InFile::new(self.file_id, node))
    }

    pub fn file_id(&self) -> FileId {
        self.file_id
    }

    pub fn kind(&self) -> SyntaxKind {
        self.kind
    }
}

impl fmt::Debug for SyntaxLoc {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyntaxLoc")
            .field("kind", &self.kind)
            .field("loc", &format!("{}::{:?}", self.file_id.index(), self.offset))
            .finish()
    }
}

pub trait SyntaxLocExt {
    fn loc(self) -> SyntaxLoc;
}

impl<T: AstNode> SyntaxLocExt for InFile<T> {
    fn loc(self) -> SyntaxLoc {
        SyntaxLoc::from_ast_node(self)
    }
}
