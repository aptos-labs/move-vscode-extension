use paths::{AbsPath, AbsPathBuf};
use std::borrow::Borrow;
use std::{fmt, ops};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ManifestPath {
    pub file: AbsPathBuf,
}

impl TryFrom<AbsPathBuf> for ManifestPath {
    type Error = AbsPathBuf;

    fn try_from(file: AbsPathBuf) -> Result<Self, Self::Error> {
        if file.parent().is_none() {
            Err(file)
        } else {
            Ok(ManifestPath { file })
        }
    }
}

impl From<ManifestPath> for AbsPathBuf {
    fn from(it: ManifestPath) -> Self {
        it.file
    }
}

impl ManifestPath {
    // Shadow `parent` from `Deref`.
    pub fn parent(&self) -> &AbsPath {
        self.file.parent().unwrap()
    }

    pub fn canonicalize(&self) -> ! {
        (**self).canonicalize()
    }

    pub fn is_rust_manifest(&self) -> bool {
        self.file.extension() == Some("rs")
    }
}

impl fmt::Display for ManifestPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.file, f)
    }
}

impl ops::Deref for ManifestPath {
    type Target = AbsPath;

    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

impl AsRef<AbsPath> for ManifestPath {
    fn as_ref(&self) -> &AbsPath {
        self.file.as_ref()
    }
}

impl AsRef<std::path::Path> for ManifestPath {
    fn as_ref(&self) -> &std::path::Path {
        self.file.as_ref()
    }
}

impl AsRef<std::ffi::OsStr> for ManifestPath {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.file.as_ref()
    }
}

impl Borrow<AbsPath> for ManifestPath {
    fn borrow(&self) -> &AbsPath {
        self.file.borrow()
    }
}
