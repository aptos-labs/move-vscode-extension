use syntax::{TextRange, TextSize};
use vfs::FileId;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct FilePosition {
    pub file_id: FileId,
    pub offset: TextSize,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct FileRange {
    pub file_id: FileId,
    pub range: TextRange,
}

/// `InFile<T>` stores a value of `T` inside a particular file/syntax tree.
///
/// Typical usages are:
///
/// * `InFile<SyntaxNode>` -- syntax node in a file
/// * `InFile<ast::FnDef>` -- ast node in a file
/// * `InFile<TextSize>` -- offset in a file
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct InFile<T> {
    pub file_id: FileId,
    pub value: T,
}

impl<T> InFile<T> {
    pub fn new(file_id: FileId, value: T) -> Self {
        Self { file_id, value }
    }

    pub fn map<F: FnOnce(T) -> U, U>(self, f: F) -> InFile<U> {
        InFile::new(self.file_id, f(self.value))
    }
}
