// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::global_state::{GlobalState, LoadPackagesRequest};
use crate::reload;
use base_db::change::FileChanges;
use parking_lot::{RwLockUpgradableReadGuard, RwLockWriteGuard};
use paths::AbsPathBuf;
use stdext::line_endings::LineEndings;

impl GlobalState {
    #[tracing::instrument(level = "info", skip(self))]
    pub(crate) fn process_pending_file_changes(&mut self) -> bool {
        let Some((mut changes, structure_changes)) = self.fetch_pending_file_changes() else {
            return false;
        };
        let needs_to_refresh_packages = structure_changes.has_changes();

        if needs_to_refresh_packages {
            tracing::info!(?needs_to_refresh_packages);
            // let n_to_show = 10;
            // if structure_changes.len() < n_to_show {
            //     tracing::info!(n_files = structure_changes.len(), changed_paths = ?structure_changes, "refreshing package roots");
            // } else {
            //     let changed_paths = structure_changes[0..n_to_show].to_vec();
            //     tracing::info!(
            //         "refreshing package roots: changed_paths = {:?} ...",
            //         changed_paths
            //     );
            // };

            let vfs = &self.vfs.read().0;
            let new_package_roots = self.package_root_config.partition_into_package_roots(vfs);
            changes.set_package_roots(new_package_roots);
        }

        let _p = tracing::info_span!("GlobalState::process_changes/apply_change").entered();
        self.analysis_host.apply_change(changes);

        if needs_to_refresh_packages {
            let _p = tracing::info_span!("GlobalState::process_changes/ws_structure_change").entered();
            // let cause = if structure_changes.len() > 5 {
            //     format!("vfs structure changes, n_files = {:?}", structure_changes.len())
            // } else {
            //     format!("vfs structure changes {:?}", structure_changes)
            // };
            // let cause = format!("vfs structure changes {}", structure_changes);
            self.load_aptos_packages_queue.request_op(
                "vfs structure changes".to_string(),
                LoadPackagesRequest {
                    force_reload_package_deps: true,
                },
            );
        }

        true
    }

    pub(crate) fn fetch_pending_file_changes(
        &mut self,
    ) -> Option<(FileChanges, WorkspaceStructureChanges)> {
        let mut changes = FileChanges::new();

        let mut vfs_lock = self.vfs.write();

        // fetch latest file changes
        let vfs = &mut vfs_lock.0;
        let changed_files = vfs.take_changes();
        if changed_files.is_empty() {
            return None;
        }

        // downgrade to read lock to allow more readers while we are normalizing text
        let vfs_lock = RwLockWriteGuard::downgrade_to_upgradable(vfs_lock);
        let vfs = &vfs_lock.0;

        let mut notable_changes = WorkspaceStructureChanges::default();
        let mut files_with_text = vec![];

        for changed_file in changed_files.into_values() {
            if let Some(changed_file_path) = vfs.file_path(changed_file.file_id).as_path() {
                let changed_file_path = changed_file_path.to_path_buf();
                match &changed_file.change {
                    vfs::Change::Create(_, _) => notable_changes.files_created.push(changed_file_path),
                    vfs::Change::Delete => notable_changes.files_deleted.push(changed_file_path),
                    vfs::Change::Modify(_, _) if reload::is_manifest_file(&changed_file_path) => {
                        notable_changes.manifests_changed.push(changed_file_path)
                    }
                    _ => (),
                }
            }

            let file_text = match changed_file.change {
                vfs::Change::Create(bytes, _) => String::from_utf8(bytes).ok(),
                vfs::Change::Modify(bytes, _) => String::from_utf8(bytes).ok(),
                _ => None,
            };

            // delay `line_endings_map` changes until we are done normalizing the text
            // this allows delaying the re-acquisition of the write lock
            files_with_text.push((
                changed_file.file_id,
                file_text.map(|it| LineEndings::normalize(it)),
            ));
        }

        let (_, line_endings_map) = &mut *RwLockUpgradableReadGuard::upgrade(vfs_lock);
        for (file_id, text_with_line_endings) in files_with_text {
            let text = match text_with_line_endings {
                Some((text, line_endings)) => {
                    line_endings_map.insert(file_id, line_endings);
                    Some(text)
                }
                None => None,
            };
            changes.change_file(file_id, text);
        }

        Some((changes, notable_changes))
    }
}

#[derive(Debug, Default)]
pub(crate) struct WorkspaceStructureChanges {
    files_created: Vec<AbsPathBuf>,
    files_deleted: Vec<AbsPathBuf>,
    manifests_changed: Vec<AbsPathBuf>,
}

impl WorkspaceStructureChanges {
    pub fn has_changes(&self) -> bool {
        !self.files_created.is_empty()
            || !self.files_deleted.is_empty()
            || !self.manifests_changed.is_empty()
    }
}
