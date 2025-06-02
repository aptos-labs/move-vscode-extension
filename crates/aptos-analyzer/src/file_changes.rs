use crate::global_state::{GlobalState, LoadPackagesRequest};
use crate::line_index::LineEndings;
use crate::reload;
use base_db::change::FileChanges;
use parking_lot::{RwLockUpgradableReadGuard, RwLockWriteGuard};
use paths::AbsPathBuf;

impl GlobalState {
    #[tracing::instrument(level = "info", skip(self))]
    pub(crate) fn process_pending_file_changes(&mut self) -> bool {
        let Some((mut changes, notable_changes)) = self.fetch_pending_file_changes() else {
            return false;
        };
        let needs_to_refresh_package_roots = !notable_changes.is_empty();
        tracing::debug!(?needs_to_refresh_package_roots);

        if needs_to_refresh_package_roots {
            let n_to_show = 10;
            if notable_changes.len() < n_to_show {
                tracing::info!(n_files = notable_changes.len(), changed_paths = ?notable_changes, "refreshing package roots");
            } else {
                let changed_paths = notable_changes[0..n_to_show].to_vec();
                tracing::info!(
                    "refreshing package roots: changed_paths = {:?} ...",
                    changed_paths
                );
            };

            let vfs = &self.vfs.read().0;
            let new_package_roots = self.package_root_config.partition_into_package_roots(vfs);
            changes.set_package_roots(new_package_roots);
        }

        let _p = tracing::info_span!("GlobalState::process_changes/apply_change").entered();
        self.analysis_host.apply_change(changes);

        let has_manifests_changes = notable_changes.iter().any(|it| reload::is_manifest_file(it));
        tracing::info!(?has_manifests_changes);

        if has_manifests_changes {
            let _p = tracing::info_span!("GlobalState::process_changes/ws_structure_change").entered();
            self.load_aptos_packages_queue.request_op(
                "manifest vfs file change".to_string(),
                LoadPackagesRequest {
                    force_reload_package_deps: true,
                },
            );
        }

        true
    }

    pub(crate) fn fetch_pending_file_changes(&mut self) -> Option<(FileChanges, Vec<AbsPathBuf>)> {
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

        let mut notable_changes = vec![];
        let mut line_endings_changes = vec![];

        for changed_file in changed_files.into_values() {
            if let Some(changed_file_path) = vfs.file_path(changed_file.file_id).as_path() {
                if changed_file.is_created_or_deleted() || reload::is_manifest_file(&changed_file_path) {
                    notable_changes.push(changed_file_path.to_path_buf());
                }
            }

            // Clear native diagnostics when their file gets deleted
            if !changed_file.exists() {
                self.diagnostics.clear_native_for(changed_file.file_id);
            }

            let file_text = match changed_file.change {
                vfs::Change::Create(bytes, _) => String::from_utf8(bytes).ok(),
                vfs::Change::Modify(bytes, _) => String::from_utf8(bytes).ok(),
                _ => None,
            };

            // delay `line_endings_map` changes until we are done normalizing the text
            // this allows delaying the re-acquisition of the write lock
            line_endings_changes.push((
                changed_file.file_id,
                file_text.map(|it| LineEndings::normalize(it)),
            ));
        }

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

        Some((changes, notable_changes))
    }
}
