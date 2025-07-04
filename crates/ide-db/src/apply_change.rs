// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::RootDatabase;
use base_db::change::FileChanges;
use salsa::{Database, Durability};

impl RootDatabase {
    pub fn request_cancellation(&mut self) {
        let _p = tracing::info_span!("RootDatabase::request_cancellation").entered();
        self.synthetic_write(Durability::LOW);
    }

    #[tracing::instrument(level = "info", skip_all)]
    pub fn apply_change(&mut self, change: FileChanges) {
        // let db_revision_before = salsa::plumbing::current_revision(self);
        self.request_cancellation();

        tracing::trace!("apply_change {:?}", change);

        // if let Some(roots) = &change.package_roots {
        //     let mut local_roots = HashSet::default();
        //     let mut library_roots = HashSet::default();
        //     for (idx, root) in roots.iter().enumerate() {
        //         let package_id = PackageId::new(self, idx as u32);
        //         if root.is_library() {
        //             library_roots.insert(package_id);
        //         } else {
        //             local_roots.insert(package_id);
        //         }
        //     }
        //     self.set_local_roots_with_durability(Arc::new(local_roots), Durability::HIGH);
        //     self.set_library_roots_with_durability(Arc::new(library_roots), Durability::HIGH);
        // }

        change.apply(self);

        // tracing::debug!(
        //     "db_revision = {:?} -> {:?}",
        //     db_revision_before,
        //     salsa::plumbing::current_revision(self)
        // );
    }
}
