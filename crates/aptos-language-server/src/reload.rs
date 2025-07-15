// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::config::FilesWatcher;
use crate::global_state::{GlobalState, LoadPackagesRequest, LoadPackagesResponse};
use crate::lsp::utils::Progress;
use crate::main_loop::Task;
use crate::op_queue::Cause;
use crate::{Config, lsp_ext};
use base_db::change::{FileChanges, ManifestFileId, PackageGraph};
use lsp_types::FileSystemWatcher;
use project_model::aptos_package::{AptosPackage, load_from_fs};
use project_model::dep_graph::collect;
use project_model::project_folders::ProjectFolders;
use std::fmt::Formatter;
use std::sync::Arc;
use std::{fmt, mem};
use stdx::format_to;
use stdx::thread::ThreadIntent;
use vfs::loader::Handle;
use vfs::{AbsPath, Vfs};

#[derive(Debug)]
pub(crate) enum FetchPackagesProgress {
    Begin,
    #[allow(unused)]
    Report(String),
    End(Vec<anyhow::Result<AptosPackage>>, bool),
}

impl fmt::Display for FetchPackagesProgress {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            FetchPackagesProgress::Begin => write!(f, "FetchPackagesProgress::Begin"),
            FetchPackagesProgress::Report(s) => write!(f, "FetchPackagesProgress::Report({s})"),
            FetchPackagesProgress::End(ps, force_reload) => {
                write!(
                    f,
                    "FetchPackagesProgress::End(n_packages={}, force_reload={})",
                    ps.len(),
                    force_reload
                )
            }
        }
    }
}

impl GlobalState {
    /// Is the server quiescent?
    ///
    /// This indicates that we've fully loaded the projects and
    /// are ready to do semantic work.
    pub(crate) fn is_projects_fully_loaded(&self) -> bool {
        self.vfs_done
            && !self.load_aptos_packages_queue.op_in_progress()
            && self.vfs_progress_config_version >= self.vfs_config_version
    }

    pub(crate) fn update_configuration(&mut self, config: Config) {
        let _p = tracing::info_span!("GlobalState::update_configuration").entered();

        let old_config = mem::replace(&mut self.config, Arc::new(config));

        if self.config.discovered_manifests() != old_config.discovered_manifests() {
            let req = LoadPackagesRequest {
                force_reload_package_deps: false,
            };
            self.load_aptos_packages_queue
                .request_op("discovered projects changed".to_owned(), req)
        }
    }

    pub(crate) fn current_status(&self) -> lsp_ext::ServerStatusParams {
        let mut status = lsp_ext::ServerStatusParams {
            health: lsp_ext::Health::Ok,
            quiescent: self.is_projects_fully_loaded(),
            message: None,
        };
        let mut message = String::new();

        if !self.config.autorefresh_on_move_toml_changes()
            && self.is_projects_fully_loaded()
            && self.load_aptos_packages_queue.op_requested()
        {
            status.health |= lsp_ext::Health::Warning;
            message.push_str("Auto-reloading is disabled and the workspace has changed, a manual workspace reload is required.\n\n");
        }

        if let Some(err) = &self.config_errors {
            status.health |= lsp_ext::Health::Warning;
            format_to!(message, "{err}\n");
        }

        if self.config.discovered_manifests().is_empty() {
            status.health |= lsp_ext::Health::Warning;
            message.push_str("Failed to discover Aptos packages in the current folder.");
        }
        if self.load_packages_error().is_err() {
            status.health |= lsp_ext::Health::Error;
            message.push_str("Failed to load some of the Aptos packages.");
            message.push_str("\n\n");
        }

        // todo: show error for the future `aptos metadata` call (see rust-analyzer)

        if !message.is_empty() {
            status.message = Some(message.trim_end().to_owned());
        }

        status
    }

    #[tracing::instrument(level = "info", skip(self))]
    pub(crate) fn load_aptos_packages_from_fs(
        &mut self,
        _cause: Cause,
        force_reload_package_deps: bool,
    ) {
        let discovered_manifests = self.config.discovered_manifests();
        tracing::info!(
            "discovered packages: {:#?}",
            discovered_manifests
                .iter()
                .map(|it| it.display_root())
                .collect::<Vec<_>>()
        );
        tracing::info!(
            "skip deps for: {:#?}",
            discovered_manifests
                .iter()
                .filter(|it| !it.resolve_deps)
                .map(|it| it.display_root())
                .collect::<Vec<_>>()
        );
        tracing::info!("send fetch_packages() to the worker thread pool");
        self.task_pool
            .handle
            .spawn_with_sender(ThreadIntent::Worker, move |sender| {
                let _p = tracing::info_span!("worker thread: fetch_packages()").entered();
                sender
                    .send(Task::FetchPackagesProgress(FetchPackagesProgress::Begin))
                    .unwrap();
                tracing::info!("load {} packages", discovered_manifests.len());
                // ACTUAL WORK: hits the filesystem directly
                let fetched_packages = load_from_fs::load_aptos_packages(discovered_manifests);
                sender
                    .send(Task::FetchPackagesProgress(FetchPackagesProgress::End(
                        fetched_packages,
                        force_reload_package_deps,
                    )))
                    .unwrap();
            });
    }

    #[tracing::instrument(level = "info", skip(self))]
    pub(crate) fn switch_packages(&mut self, cause: Cause) {
        let Some(LoadPackagesResponse {
            packages_from_fs,
            force_reload_package_deps,
        }) = self.load_aptos_packages_queue.last_op_result()
        else {
            return;
        };

        let switching_from_empty_workspace = self.all_packages.is_empty();
        tracing::info!(%switching_from_empty_workspace, %force_reload_package_deps);

        if let Err(load_packages_error) = self.load_packages_error() {
            tracing::info!(%load_packages_error);
            // already have a workspace, let's keep it instead of loading invalid state
            if !switching_from_empty_workspace {
                if *force_reload_package_deps {
                    self.recreate_package_graph(
                        format!("fetch packages error while handling {:?}", cause),
                        // switching_from_empty_workspace,
                    );
                }
                return;
            }
        }

        let packages_from_fs = packages_from_fs
            .iter()
            .filter_map(|res| res.as_ref().ok().cloned())
            .collect::<Vec<_>>();
        tracing::info!(
            "switch to packages: {:#?}",
            packages_from_fs
                .iter()
                .map(|it| it.display_root())
                .collect::<Vec<_>>()
        );

        // let same_packages = packages_from_fs.len() == self.all_packages.len()
        //     && packages_from_fs
        //         .iter()
        //         .zip(self.all_packages.iter())
        //         .all(|(l, r)| l.eq(r));
        //
        // if same_packages {
        //     if switching_from_empty_workspace {
        //         // Switching from empty to empty is a no-op
        //         return;
        //     }
        //     tracing::info!(?force_reload_deps, "packages are unchanged");
        //     if *force_reload_deps {
        //         self.recreate_package_graph(cause, switching_from_empty_workspace);
        //     }
        //     // Unchanged workspaces, nothing to do here
        //     return;
        // }

        let project_folders = ProjectFolders::new(&packages_from_fs);
        let watch = match self.config.files().watcher {
            FilesWatcher::Server => project_folders.watch,
            FilesWatcher::Client => {
                self.setup_client_file_watchers(&packages_from_fs);
                vec![]
            }
        };
        self.vfs_config_version += 1;
        // starts the process of vfs refresh
        self.vfs_loader.handle.set_config(vfs::loader::Config {
            load: project_folders.load,
            watch,
            version: self.vfs_config_version,
        });
        self.package_root_config = project_folders.package_root_config;
        self.all_packages = Arc::new(packages_from_fs);

        tracing::info!("vfs_refresh scheduled");
    }

    fn setup_client_file_watchers(&mut self, packages: &[AptosPackage]) {
        let local_folder_roots = packages
            .iter()
            .filter(|it| it.is_local())
            .map(|pkg| pkg.to_folder_root());

        let mut watchers: Vec<FileSystemWatcher> = if self
            .config
            .did_change_watched_files_relative_pattern_support()
        {
            // When relative patterns are supported by the client, prefer using them
            local_folder_roots
                .flat_map(|package_folder_root| {
                    package_folder_root
                        .source_dirs()
                        .iter()
                        .map(|it| (it.to_owned(), "**/*.move"))
                        .collect::<Vec<_>>()
                })
                .map(|(base, pat)| FileSystemWatcher {
                    glob_pattern: lsp_types::GlobPattern::Relative(lsp_types::RelativePattern {
                        base_uri: lsp_types::OneOf::Right(lsp_types::Url::from_file_path(base).unwrap()),
                        pattern: pat.to_owned(),
                    }),
                    kind: None,
                })
                .collect()
        } else {
            // When they're not, integrate the base to make them into absolute patterns
            local_folder_roots
                .flat_map(|folder_root| {
                    folder_root
                        .source_dirs()
                        .iter()
                        .map(|base| format!("{base}**/*.move"))
                        .collect::<Vec<_>>()
                })
                .map(|glob_pattern| FileSystemWatcher {
                    glob_pattern: lsp_types::GlobPattern::String(glob_pattern),
                    kind: None,
                })
                .collect()
        };

        watchers.extend(
            packages
                .iter()
                .map(|pkg| pkg.manifest_path())
                .map(|glob_pattern| FileSystemWatcher {
                    glob_pattern: lsp_types::GlobPattern::String(glob_pattern.to_string()),
                    kind: None,
                }),
        );

        let registration_options = lsp_types::DidChangeWatchedFilesRegistrationOptions { watchers };
        let registration = lsp_types::Registration {
            id: "workspace/didChangeWatchedFiles".to_owned(),
            method: "workspace/didChangeWatchedFiles".to_owned(),
            register_options: Some(serde_json::to_value(registration_options).unwrap()),
        };
        self.send_request::<lsp_types::request::RegisterCapability>(
            lsp_types::RegistrationParams {
                registrations: vec![registration],
            },
            |_, _| (),
        );
    }

    #[tracing::instrument(level = "info", skip(self))]
    pub(crate) fn recreate_package_graph(&mut self, cause: String /*, initial_build: bool*/) {
        let progress_title = "Reloading Aptos packages";
        self.report_progress(
            progress_title,
            Progress::Begin,
            Some(format!("after '{:?}'", cause)),
            None,
            None,
        );
        let package_graph = {
            let packages = self.all_packages.as_slice();
            // if initial_build {
            //     // with initial build, vfs might not be ready, so we need to load files manually
            //     let dep_graph = {
            //         let vfs = &mut self.vfs.write().0;
            //         collect_initial(packages, vfs)
            //     };
            //     // apply Move.toml changes bypassing the packages refresh
            //     if let Some((changes, _)) = self.fetch_pending_file_changes() {
            //         tracing::info!(
            //             ?changes,
            //             "initial_build=true: apply changes for Move.toml files from the sync vfs access"
            //         );
            //         self.analysis_host.apply_change(changes);
            //     }
            //     dep_graph
            // } else {
            let vfs = &self.vfs.read().0;
            let mut load = |path: &AbsPath| {
                tracing::debug!(?path, "load from vfs");
                vfs.file_id(&vfs::VfsPath::from(path.to_path_buf()))
                    .map(|it| it.0)
            };
            collect(packages, &mut load)
            // }
        };
        self.report_progress(progress_title, Progress::End, None, None, None);

        let Some(package_graph) = package_graph else {
            tracing::info!("cannot reload package dep graph, vfs is not ready yet");
            return;
        };

        {
            let vfs = &self.vfs.read().0;
            trace_dependencies(&package_graph, vfs);
        }

        let mut changes = FileChanges::new();
        changes.set_package_graph(package_graph);
        self.analysis_host.apply_change(changes);
    }

    pub(super) fn load_packages_error(&self) -> Result<(), String> {
        let mut buf = String::new();

        let Some(LoadPackagesResponse { packages_from_fs, .. }) =
            self.load_aptos_packages_queue.last_op_result()
        else {
            return Ok(());
        };

        if packages_from_fs.is_empty() {
            format_to!(buf, "aptos-language-server failed to find any packages");
        } else {
            for package_from_fs in packages_from_fs {
                if let Err(load_err) = package_from_fs {
                    format_to!(
                        buf,
                        "aptos-language-server failed to load package: {:#}\n",
                        load_err
                    );
                }
            }
        }

        if buf.is_empty() {
            return Ok(());
        }
        Err(buf)
    }
}

pub(crate) fn is_manifest_file(changed_file_path: &AbsPath) -> bool {
    let changed_file_name = match changed_file_path.file_name() {
        Some(it) => it,
        None => return false,
    };
    changed_file_name == "Move.toml"
}

fn trace_dependencies(package_entries: &PackageGraph, vfs: &Vfs) {
    for (package_manifest_id, package_metadata) in package_entries {
        let main_package = dir_file_name(vfs, *package_manifest_id).unwrap_or("<empty>".to_string());
        let dep_names = package_metadata
            .dep_manifest_ids
            .iter()
            .map(|it| dir_file_name(vfs, *it).unwrap_or("<empty>".to_string()))
            .collect::<Vec<_>>();
        if package_metadata.resolve_deps {
            tracing::info!(?main_package, ?dep_names);
        } else {
            tracing::info!(?main_package, ?dep_names, resolve_deps = false);
        }
    }
}

fn dir_file_name(vfs: &Vfs, file_id: ManifestFileId) -> Option<String> {
    let manifest_path = vfs.file_path(file_id);
    let root_path = manifest_path.as_path()?.to_path_buf().parent()?.to_path_buf();
    root_path.file_name().map(|it| it.to_string())
}
