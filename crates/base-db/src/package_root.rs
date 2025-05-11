use vfs::file_set::FileSet;
use vfs::{AnchoredPath, FileId, VfsPath};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackageId(pub u32);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageRoot {
    pub file_set: FileSet,
    /// Sysroot or crates.io library.
    ///
    /// Libraries are considered mostly immutable, this assumption is used to
    /// optimize salsa's query structure
    pub is_library: bool,
}

impl PackageRoot {
    pub fn new_local(file_set: FileSet) -> PackageRoot {
        PackageRoot { file_set, is_library: false }
    }

    pub fn new_library(file_set: FileSet) -> PackageRoot {
        PackageRoot { file_set, is_library: true }
    }

    pub fn path_for_file(&self, file: &FileId) -> Option<&VfsPath> {
        self.file_set.path_for_file(file)
    }

    pub fn file_for_path(&self, path: &VfsPath) -> Option<&FileId> {
        self.file_set.file_for_path(path)
    }

    pub fn resolve_path(&self, path: AnchoredPath<'_>) -> Option<FileId> {
        self.file_set.resolve_path(path)
    }
}
