// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use paths::{AbsPath, AbsPathBuf};
use std::borrow::Borrow;
use std::ffi::OsString;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::{fmt, fs, ops};

pub fn is_move_toml(file_name: OsString) -> bool {
    if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
        file_name.to_ascii_lowercase() == "move.toml"
    } else {
        file_name == "Move.toml"
    }
}

#[derive(Debug, Clone, Eq, Ord, PartialOrd)]
pub struct ManifestPath {
    pub file: AbsPathBuf,
}

impl Hash for ManifestPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
            self.file.to_string().to_lowercase().hash(state)
        } else {
            self.file.hash(state)
        }
    }
}

impl PartialEq for ManifestPath {
    fn eq(&self, other: &Self) -> bool {
        if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
            self.file.to_string().to_lowercase() == other.file.to_string().to_lowercase()
        } else {
            self.file == other.file
        }
    }
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
    pub fn new(move_toml_file: AbsPathBuf) -> ManifestPath {
        let file_name = move_toml_file.file_name().unwrap_or_default();
        assert!(
            is_move_toml(file_name.into()),
            "project root must point to a Move.toml file: {move_toml_file}"
        );
        Self { file: move_toml_file }
    }

    // Shadow `parent` from `Deref`.
    pub fn content_root(&self) -> AbsPathBuf {
        self.file.parent().unwrap().to_path_buf()
    }

    pub fn canonical_root(&self) -> PathBuf {
        let content_root = self.content_root();
        fs::canonicalize(&content_root).ok().unwrap_or(PathBuf::new())
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
