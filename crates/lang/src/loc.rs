use crate::InFile;
use base_db::SourceRootDatabase;
use parser::SyntaxKind;
use std::fmt;
use std::fmt::Formatter;
use syntax::algo::ancestors_at_offset;
use syntax::{AstNode, SyntaxNode, TextSize};
use vfs::FileId;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct SyntaxLoc {
    file_id: FileId,
    node_end: TextSize,
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
            node_end: range_start,
            kind,
        }
    }

    pub fn cast<T: AstNode>(self, db: &dyn SourceRootDatabase) -> Option<InFile<T>> {
        let file = db.parse(self.file_id).tree();
        let ancestors_at_offset = ancestors_at_offset(file.syntax(), self.node_end);
        for ancestor in ancestors_at_offset {
            if ancestor.text_range().end() == self.node_end {
                if let Some(node) = T::cast(ancestor) {
                    return Some(InFile::new(self.file_id, node));
                }
            }
        }
        None
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
        f.debug_tuple("Loc")
            .field(&format!("{}::{:?}", self.file_id.index(), self.node_end))
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
