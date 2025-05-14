use vfs::file_set::FileSet;
use vfs::{FileId, VfsPath};

#[salsa_macros::interned(no_lifetime)]
pub struct PackageId {
    pub idx: u32,
    pub root_dir: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageRoot {
    pub file_set: FileSet,
    /// Sysroot or crates.io library.
    ///
    /// Libraries are considered mostly immutable, this assumption is used to
    /// optimize salsa's query structure
    pub is_library: bool,
    pub root_dir: Option<String>,
}

impl PackageRoot {
    pub fn new_local(file_set: FileSet, package_name: Option<String>) -> PackageRoot {
        PackageRoot {
            file_set,
            is_library: false,
            root_dir: package_name,
        }
    }

    pub fn new_library(file_set: FileSet, package_name: Option<String>) -> PackageRoot {
        PackageRoot {
            file_set,
            is_library: true,
            root_dir: package_name,
        }
    }

    pub fn path_for_file(&self, file: &FileId) -> Option<&VfsPath> {
        self.file_set.path_for_file(file)
    }

    pub fn file_for_path(&self, path: &VfsPath) -> Option<&FileId> {
        self.file_set.file_for_path(path)
    }
}
