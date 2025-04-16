use crate::{AstNode, SyntaxNode, TextRange, TextSize};
use parser::SyntaxKind;
use std::borrow::Borrow;
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

    pub fn with_value<U>(&self, value: U) -> InFile<U> {
        InFile::new(self.file_id, value)
    }

    pub fn as_ref(&self) -> InFile<&T> {
        self.with_value(&self.value)
    }

    pub fn borrow<U>(&self) -> InFile<&U>
    where
        T: Borrow<U>,
    {
        self.with_value(self.value.borrow())
    }

    pub fn unpack(self) -> (FileId, T) {
        let InFile { file_id, value } = self;
        (file_id, value)
    }

    pub fn unpack_ref(&self) -> (FileId, &T) {
        let InFile { file_id, value } = self;
        (*file_id, value)
    }

    pub fn map<F: FnOnce(T) -> U, U>(self, f: F) -> InFile<U> {
        InFile::new(self.file_id, f(self.value))
    }

    pub fn and_then<F: FnOnce(T) -> Option<U>, U>(self, f: F) -> Option<InFile<U>> {
        f(self.value).map(|value| InFile::new(self.file_id, value))
    }

    pub fn map_into<U: From<T>>(self) -> InFile<U> {
        self.map(|it| it.into())
    }
}

impl<T: AstNode> InFile<T> {
    pub fn kind(&self) -> SyntaxKind {
        self.value.syntax().kind()
    }

    pub fn cast_into<IntoT: AstNode>(self) -> Option<InFile<IntoT>> {
        let InFile { file_id, value } = self;
        let value = IntoT::cast(value.syntax().clone())?;
        Some(InFile::new(file_id, value))
    }

    pub fn cast_into_ref<IntoT: AstNode>(&self) -> Option<InFile<IntoT>> {
        let InFile { file_id, value } = self;
        let value = IntoT::cast(value.syntax().clone())?;
        Some(InFile::new(*file_id, value))
    }

    pub fn file_range(&self) -> FileRange {
        FileRange {
            file_id: self.file_id,
            range: self.value.syntax().text_range(),
        }
    }
}

impl InFile<SyntaxNode> {
    pub fn syntax_cast<T: AstNode>(&self) -> Option<InFile<T>> {
        let InFile { file_id, value } = self.clone();
        let value = T::cast(value)?;
        Some(InFile::new(file_id, value))
    }
}

impl<T: AstNode> InFile<Vec<T>> {
    pub fn flatten(self) -> Vec<InFile<T>> {
        let (file_id, vec) = self.unpack();
        vec.into_iter().map(|it| it.in_file(file_id)).collect()
    }
}

impl<T: AstNode> InFile<T> {
    pub fn syntax_text(&self) -> String {
        self.value.syntax().text().to_string()
    }
}

// pub trait InFileInto<U> {
//     fn in_file_into(self) -> InFile<U>;
// }

// impl<T> InFile<T> {
//     pub fn in_file_into<U: From<T>>(self) -> InFile<U> {
//         self.map(|it| it.into())
//     }
// }
// impl<T, U> InFileInto<U> for InFile<T>
// where
//     U: From<T>,
// {
//     fn in_file_into(self) -> InFile<U> {
//         self.map(|it| it.into())
//     }
// }

pub trait InFileExt {
    type Node;
    fn in_file(self, file_id: FileId) -> InFile<Self::Node>;
}

impl<T: AstNode> InFileExt for T {
    type Node = T;
    fn in_file(self, file_id: FileId) -> InFile<T> {
        // AstNode clone is Rc::clone()
        InFile::new(file_id, self)
    }
}

pub trait OptionInFileExt {
    type Node;
    fn opt_in_file(self, file_id: FileId) -> Option<InFile<Self::Node>>;
}

impl<T: AstNode> OptionInFileExt for Option<T> {
    type Node = T;
    fn opt_in_file(self, file_id: FileId) -> Option<InFile<T>> {
        let v = self?;
        Some(InFile::new(file_id, v))
    }
}

pub trait InFileVecExt {
    type Node;
    fn wrapped_in_file(self, file_id: FileId) -> Vec<InFile<Self::Node>>;
}

impl<T: AstNode> InFileVecExt for Vec<T> {
    type Node = T;
    fn wrapped_in_file(self, file_id: FileId) -> Vec<InFile<T>> {
        self.into_iter().map(|node| node.in_file(file_id)).collect()
    }
}
