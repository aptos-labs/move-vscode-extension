use camino::Utf8PathBuf;
use vfs::file_set::FileSet;
use vfs::{AbsPathBuf, FileId, Vfs, VfsPath};

#[salsa_macros::interned(no_lifetime, debug)]
pub struct PackageId {
    pub idx: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackageKind {
    Local,
    /// Sysroot or crates.io library.
    ///
    /// Libraries are considered mostly immutable, this assumption is used to
    /// optimize salsa's query structure
    Library,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageRoot {
    pub file_set: FileSet,
    pub kind: PackageKind,
    pub manifest_file_id: Option<FileId>,
}

impl PackageRoot {
    pub fn new(file_set: FileSet, kind: PackageKind, manifest_file_id: Option<FileId>) -> Self {
        PackageRoot {
            file_set,
            kind,
            manifest_file_id,
        }
    }

    pub fn is_library(&self) -> bool {
        self.kind == PackageKind::Library
    }

    pub fn root_dir(&self, vfs: &Vfs) -> Option<AbsPathBuf> {
        let manifest_file_id = self.manifest_file_id?;
        if !vfs.exists(manifest_file_id) {
            return None;
        }
        let root_dir = vfs.file_path(manifest_file_id).parent()?;
        root_dir.as_path().map(|it| it.to_path_buf())
    }

    pub fn root_dir_name(&self, vfs: &Vfs) -> Option<String> {
        self.root_dir(vfs)
            .and_then(|it| it.file_name().map(|n| n.to_string()))
    }

    pub fn path_for_file(&self, file: &FileId) -> Option<&VfsPath> {
        self.file_set.path_for_file(file)
    }

    pub fn file_for_path(&self, path: &VfsPath) -> Option<&FileId> {
        self.file_set.file_for_path(path)
    }
}
