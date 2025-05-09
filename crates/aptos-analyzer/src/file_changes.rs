use crate::global_state::{FetchPackagesRequest, GlobalState};
use crate::line_index::LineEndings;
use crate::reload;
use base_db::change::FileChanges;
use parking_lot::{RwLockUpgradableReadGuard, RwLockWriteGuard};
use paths::AbsPathBuf;

impl GlobalState {
    #[tracing::instrument(level = "info", skip(self))]
    pub(crate) fn process_pending_file_changes(&mut self) -> bool {
        let Some((mut changes, important_changes)) = self.fetch_latest_file_changes() else {
            return false;
        };
        let needs_to_refresh_packages = !important_changes.is_empty();
        tracing::debug!(?needs_to_refresh_packages);

        if needs_to_refresh_packages {
            let n_to_show = 10;
            if important_changes.len() < n_to_show {
                tracing::info!(n_files = important_changes.len(), paths = ?important_changes);
            } else {
                let changes = important_changes[0..n_to_show].to_vec();
                tracing::info!("paths = {:?} ...", changes);
            };
        }

        if needs_to_refresh_packages {
            let vfs = &self.vfs.read().0;
            let new_package_roots = self.package_root_config.partition_into_roots(vfs);
            changes.set_package_roots(new_package_roots);
        }

        let _p = tracing::info_span!("GlobalState::process_changes/apply_change").entered();
        self.analysis_host.apply_change(changes);

        if needs_to_refresh_packages {
            let _p = tracing::info_span!("GlobalState::process_changes/ws_structure_change").entered();
            self.fetch_packages_queue.request_op(
                "workspace vfs file change".to_string(),
                FetchPackagesRequest { force_reload_deps: true },
            );
        }

        true
    }

    fn fetch_latest_file_changes(&mut self) -> Option<(FileChanges, Vec<AbsPathBuf>)> {
        let mut changes = FileChanges::new();

        let mut vfs_lock = self.vfs.write();
        // fetch latest file changes
        let changed_files = vfs_lock.0.take_changes();
        if changed_files.is_empty() {
            return None;
        }

        // downgrade to read lock to allow more readers while we are normalizing text
        let vfs_lock = RwLockWriteGuard::downgrade_to_upgradable(vfs_lock);
        let vfs: &vfs::Vfs = &vfs_lock.0;

        let mut important_changes = vec![];
        let mut line_endings_changes = vec![];

        for changed_file in changed_files.into_values() {
            let changed_file_vfs_path = vfs.file_path(changed_file.file_id);

            if let Some(changed_file_path) = changed_file_vfs_path.as_path() {
                if changed_file.is_created_or_deleted() {
                    important_changes.push(changed_file_path.to_path_buf());
                    // continue;
                }
                // refresh_packages |= changed_file.is_created_or_deleted();
                else if reload::should_refresh_for_file_change(&changed_file_path) {
                    tracing::trace!(?changed_file_path, kind = ?changed_file.kind(), "refreshing for a change");
                    important_changes.push(changed_file_path.to_path_buf());
                    // continue;
                    // refresh_packages |= true;
                }
            }

            // Clear native diagnostics when their file gets deleted
            if !changed_file.exists() {
                self.diagnostics.clear_native_for(changed_file.file_id);
            }

            let text_with_line_endings = match changed_file.change {
                vfs::Change::Create(v, _) | vfs::Change::Modify(v, _) => {
                    String::from_utf8(v).ok().map(|text| LineEndings::normalize(text))
                }
                _ => None,
            };

            // delay `line_endings_map` changes until we are done normalizing the text
            // this allows delaying the re-acquisition of the write lock
            line_endings_changes.push((changed_file.file_id, text_with_line_endings));
        }

        let _p = tracing::info_span!("upgrade lock to exclusive write lock").entered();
        let (_, line_endings_map) = &mut *RwLockUpgradableReadGuard::upgrade(vfs_lock);

        for (file_id, text_with_line_endings) in line_endings_changes {
            let text = match text_with_line_endings {
                Some((text, line_endings)) => {
                    line_endings_map.insert(file_id, line_endings);
                    Some(text)
                }
                None => None,
            };
            changes.change_file(file_id, text);
        }

        Some((changes, important_changes))
    }
}
