// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

//! In-memory document information.

use std::collections::HashMap;

use vfs::VfsPath;

/// Holds the set of in-memory documents.
///
/// For these document, their true contents is maintained by the client. It
/// might be different from what's on disk.
#[derive(Default, Clone)]
pub(crate) struct OpenedFiles {
    files: HashMap<VfsPath, DocumentData>,
}

impl OpenedFiles {
    pub(crate) fn contains(&self, path: &VfsPath) -> bool {
        self.files.contains_key(path)
    }

    pub(crate) fn insert(&mut self, path: VfsPath, data: DocumentData) -> Result<(), ()> {
        match self.files.insert(path, data) {
            Some(_) => Err(()),
            None => Ok(()),
        }
    }

    pub(crate) fn remove(&mut self, path: &VfsPath) -> Result<(), ()> {
        match self.files.remove(path) {
            Some(_) => Ok(()),
            None => Err(()),
        }
    }

    pub(crate) fn get(&self, path: &VfsPath) -> Option<&DocumentData> {
        self.files.get(path)
    }

    pub(crate) fn get_mut(&mut self, path: &VfsPath) -> Option<&mut DocumentData> {
        // NB: don't set `self.added_or_removed` here, as that purposefully only
        // tracks changes to the key set.
        self.files.get_mut(path)
    }
}

/// Information about a document that the Language Client
/// knows about.
/// Its lifetime is driven by the textDocument/didOpen and textDocument/didClose
/// client notifications.
#[derive(Debug, Clone)]
pub(crate) struct DocumentData {
    pub(crate) version: i32,
    pub(crate) data: Vec<u8>,
}

impl DocumentData {
    pub(crate) fn new(version: i32, data: Vec<u8>) -> Self {
        DocumentData { version, data }
    }
}
