use crate::InFile;
use base_db::SourceRootDatabase;
use parser::SyntaxKind;
use std::fmt;
use std::fmt::Formatter;
use syntax::algo::ancestors_at_offset;
use syntax::{AstNode, TextSize};
use vfs::FileId;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct SyntaxLoc {
    file_id: FileId,
    node_offset: TextSize,
    kind: SyntaxKind,
}

impl SyntaxLoc {
    pub fn from_ast_node<T: AstNode>(file_id: FileId, node: &T) -> Self {
        let range_start = node.syntax().text_range().end();
        let kind = node.syntax().kind();
        SyntaxLoc {
            file_id: file_id.to_owned(),
            node_offset: range_start,
            kind,
        }
    }

    pub fn to_ast<T: AstNode>(&self, db: &dyn SourceRootDatabase) -> Option<InFile<T>> {
        let file = db.parse(self.file_id).tree();
        if !file.syntax().text_range().contains_inclusive(self.node_offset) {
            tracing::error!(
                "stale cache error: {:?} offset is outside of the file range {:?}",
                self.node_offset,
                file.syntax().text_range()
            );
            return None;
        }
        let ancestors_at_offset = ancestors_at_offset(file.syntax(), self.node_offset);
        for ancestor in ancestors_at_offset {
            if ancestor.text_range().end() == self.node_offset {
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
            .field(&format!("{}::{:?}", self.file_id.index(), self.node_offset))
            .finish()
    }
}

pub trait SyntaxLocFileExt {
    fn loc(&self) -> SyntaxLoc;
}

impl<T: AstNode> SyntaxLocFileExt for InFile<T> {
    fn loc(&self) -> SyntaxLoc {
        SyntaxLoc::from_ast_node(self.file_id, &self.value)
    }
}

pub trait SyntaxLocNodeExt {
    fn loc(&self, file_id: FileId) -> SyntaxLoc;
}

impl<T: AstNode> SyntaxLocNodeExt for T {
    fn loc(&self, file_id: FileId) -> SyntaxLoc {
        SyntaxLoc::from_ast_node(file_id, self)
    }
}
