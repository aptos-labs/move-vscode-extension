use crate::config::FilesWatcher;
use crate::flycheck::FlycheckHandle;
use crate::global_state::{FetchPackagesRequest, FetchPackagesResponse, GlobalState};
use crate::main_loop::Task;
use crate::op_queue::Cause;
use crate::project_folders::ProjectFolders;
use crate::{Config, lsp_ext};
use base_db::change::{DepGraph, FileChanges};
use lsp_types::FileSystemWatcher;
use project_model::DiscoveredManifest;
use project_model::aptos_package::AptosPackage;
use project_model::manifest_path::ManifestPath;
use std::fmt::Formatter;
use std::sync::Arc;
use std::time::Duration;
use std::{fmt, mem};
use stdx::format_to;
use stdx::itertools::Itertools;
use stdx::thread::ThreadIntent;
use vfs::AbsPath;

#[derive(Debug)]
pub(crate) enum FetchPackagesProgress {
    Begin,
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
    pub(crate) fn is_quiescent(&self) -> bool {
        self.vfs_done
            && !self.fetch_packages_queue.op_in_progress()
            && self.vfs_progress_config_version >= self.vfs_config_version
    }

    /// Is the server ready to respond to analysis dependent LSP requests?
    ///
    /// Unlike `is_quiescent`, this returns false when we're indexing
    /// the project, because we're holding the salsa lock and cannot
    /// respond to LSP requests that depend on salsa data.
    fn is_fully_ready(&self) -> bool {
        self.is_quiescent() /* && !self.prime_caches_queue.op_in_progress()*/
    }

    pub(crate) fn update_configuration(&mut self, config: Config) {
        let _p = tracing::info_span!("GlobalState::update_configuration").entered();
        let old_config = mem::replace(&mut self.config, Arc::new(config));

        if self.config.discovered_manifests() != old_config.discovered_manifests() {
            let req = FetchPackagesRequest { force_reload_deps: false };
            self.fetch_packages_queue
                .request_op("discovered projects changed".to_owned(), req)
        } else if self.config.flycheck_config() != old_config.flycheck_config() {
            self.reload_flycheck();
        }
    }

    pub(crate) fn current_status(&self) -> lsp_ext::ServerStatusParams {
        let mut status = lsp_ext::ServerStatusParams {
            health: lsp_ext::Health::Ok,
            quiescent: self.is_fully_ready(),
            message: None,
        };
        let mut message = String::new();

        if !self.config.cargo_autoreload_config()
            && self.is_quiescent()
            && self.fetch_packages_queue.op_requested()
        {
            status.health |= lsp_ext::Health::Warning;
            message.push_str("Auto-reloading is disabled and the workspace has changed, a manual workspace reload is required.\n\n");
        }

        if let Some(err) = &self.config_errors {
            status.health |= lsp_ext::Health::Warning;
            format_to!(message, "{err}\n");
        }

        if let Some(err) = &self.last_flycheck_error {
            status.health |= lsp_ext::Health::Warning;
            message.push_str(err);
            message.push('\n');
        }

        if self.config.discovered_manifests().is_empty() {
            status.health |= lsp_ext::Health::Warning;
            message.push_str("Failed to discover Aptos packages in the current folder.");
        }
        if self.fetch_workspace_error().is_err() {
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
    pub(crate) fn fetch_packages(&mut self, _cause: Cause, force_reload_deps: bool) {
        let discovered_manifests = self.config.discovered_manifests();
        {
            let mut with_resolve = vec![];
            let mut without_resolve = vec![];
            for manifest in discovered_manifests.clone() {
                if manifest.resolve_deps {
                    with_resolve.push(manifest.move_toml_file.to_path_buf());
                } else {
                    without_resolve.push(manifest.move_toml_file.to_path_buf());
                }
            }
            tracing::info!(manifests_with_resolution = ?with_resolve);
            tracing::info!(manifests_without_deps = ?without_resolve);
        }
        tracing::info!("schedule to the worker thread pool");
        self.task_pool
            .handle
            .spawn_with_sender(ThreadIntent::Worker, move |sender| {
                let _p = tracing::info_span!("on the worker thread: load packages").entered();
                sender
                    .send(Task::FetchPackagesProgress(FetchPackagesProgress::Begin))
                    .unwrap();
                let discovered_packages = {
                    discovered_manifests
                        .iter()
                        .map(|discovered| {
                            sender
                                .clone()
                                .send(Task::FetchPackagesProgress(FetchPackagesProgress::Report(
                                    format!("Fetching {}", discovered.move_toml_file.to_path_buf()),
                                )))
                                .unwrap();
                            let manifest_path =
                                ManifestPath::new(discovered.move_toml_file.to_path_buf());
                            AptosPackage::load(&manifest_path, discovered.resolve_deps)
                        })
                        .collect::<Vec<_>>()
                };
                sender
                    .send(Task::FetchPackagesProgress(FetchPackagesProgress::End(
                        discovered_packages,
                        force_reload_deps,
                    )))
                    .unwrap();
            });
    }

    #[tracing::instrument(level = "info", skip(self))]
    pub(crate) fn switch_workspaces(&mut self, cause: Cause) {
        let Some(FetchPackagesResponse { packages, force_reload_deps }) =
            self.fetch_packages_queue.last_op_result()
        else {
            return;
        };
        let switching_from_empty_workspace = self.main_packages.is_empty();

        tracing::info!(?force_reload_deps, %switching_from_empty_workspace);
        if self.fetch_workspace_error().is_err() && !switching_from_empty_workspace {
            if *force_reload_deps {
                self.reload_package_deps(format!("fetch workspace error while handling {:?}", cause));
            }
            // It only makes sense to switch to a partially broken workspace
            // if we don't have any workspace at all yet.
            return;
        }

        let packages = packages
            .iter()
            .filter_map(|res| res.as_ref().ok().cloned())
            .collect::<Vec<_>>();

        let same_packages = packages.len() == self.main_packages.len()
            && packages
                .iter()
                .zip(self.main_packages.iter())
                .all(|(l, r)| l.eq(r));

        if same_packages {
            if switching_from_empty_workspace {
                // Switching from empty to empty is a no-op
                return;
            }
            if *force_reload_deps {
                tracing::info!(?force_reload_deps, "workspaces are unchanged");
                self.reload_package_deps(cause);
            }
            // Unchanged workspaces, nothing to do here
            return;
        }

        self.main_packages = Arc::new(packages);

        if let FilesWatcher::Client = self.config.files().watcher {
            self.setup_client_file_watchers();
        }

        let files_config = self.config.files();
        let project_folders = ProjectFolders::new(&self.main_packages /*&files_config.exclude*/);

        let watch = match files_config.watcher {
            FilesWatcher::Client => vec![],
            FilesWatcher::Server => project_folders.watch,
        };
        self.vfs_config_version += 1;
        self.loader.handle.set_config(vfs::loader::Config {
            load: project_folders.load,
            watch,
            version: self.vfs_config_version,
        });

        tracing::info!(
            "project_folders.package_root_config = {:#?}",
            project_folders.package_root_config
        );
        self.package_root_config = project_folders.package_root_config;

        self.reload_package_deps(cause);

        tracing::info!("did switch workspaces");
    }

    fn setup_client_file_watchers(&mut self) {
        let package_folders = self
            .main_packages
            .iter()
            .flat_map(|pkg| pkg.to_folder_roots())
            .filter(|it| it.is_local);

        let mut watchers: Vec<FileSystemWatcher> = if self
            .config
            .did_change_watched_files_relative_pattern_support()
        {
            // When relative patterns are supported by the client, prefer using them
            package_folders
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
            package_folders
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
            self.main_packages
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
    fn reload_package_deps(&mut self, cause: String) {
        let progress_title = "Reloading Aptos packages";
        self.report_progress(
            progress_title,
            crate::lsp::utils::Progress::Begin,
            Some(format!("after {:?}", cause)),
            None,
            None,
        );

        let Some(dep_graph) = self.collect_dep_graph() else {
            // vfs is not yet ready, dep graph is not valid
            tracing::info!("cannot reload package dep graph, vfs is not ready yet");
            self.report_progress(progress_title, crate::lsp::utils::Progress::End, None, None, None);
            return;
        };

        let mut change = FileChanges::new();
        {
            let vfs = &self.vfs.read().0;
            let roots = self.package_root_config.partition_into_roots(vfs);
            change.set_package_roots(roots);
            // depends on roots being available
            change.set_package_graph(dep_graph);
        }
        self.analysis_host.apply_change(change);

        self.report_progress(progress_title, crate::lsp::utils::Progress::End, None, None, None);

        self.process_pending_file_changes();
        self.reload_flycheck();
    }

    #[tracing::instrument(level = "info", skip(self))]
    fn collect_dep_graph(&mut self) -> Option<DepGraph> {
        let mut global_dep_graph = DepGraph::default();

        let vfs = &self.vfs.read().0;
        let mut load = |path: &AbsPath| {
            vfs.file_id(&vfs::VfsPath::from(path.to_path_buf()))
                .map(|it| it.0)
        };

        for main_package in self.main_packages.iter() {
            let dep_graph = main_package.to_dep_graph(&mut load)?;
            global_dep_graph.extend(dep_graph);
        }

        Some(global_dep_graph)
    }

    pub(super) fn fetch_workspace_error(&self) -> Result<(), String> {
        let mut buf = String::new();

        let Some(FetchPackagesResponse { packages: workspaces, .. }) =
            self.fetch_packages_queue.last_op_result()
        else {
            return Ok(());
        };

        if workspaces.is_empty() {
            format_to!(buf, "aptos-analyzer failed to fetch workspace");
        } else {
            for ws in workspaces {
                if let Err(err) = ws {
                    format_to!(buf, "aptos-analyzer failed to load workspace: {:#}\n", err);
                }
            }
        }

        if buf.is_empty() {
            return Ok(());
        }

        Err(buf)
    }

    fn reload_flycheck(&mut self) {
        let _p = tracing::info_span!("GlobalState::reload_flycheck").entered();
        let config = self.config.flycheck_config();
        if config.is_none() {
            self.flycheck = Arc::from_iter([]);
            return;
        }

        let config = config.unwrap();
        if !config.enabled {
            tracing::info!("stop reloading flycheck as it's disabled in settings");
            return;
        }

        let sender = self.flycheck_sender.clone();
        self.flycheck = self
            .main_packages
            .iter()
            .enumerate()
            .filter_map(|(id, ws)| Some((id, ws.content_root(), ws.manifest_path())))
            .map(|(ws_id, ws_root, _)| {
                FlycheckHandle::spawn(ws_id, sender.clone(), config.clone(), ws_root.to_path_buf())
            })
            .collect::<Vec<_>>()
            .into();
    }
}

fn dedup(packages: &mut Vec<anyhow::Result<AptosPackage>>) {
    let mut i = 0;
    while i < packages.len() {
        if let Ok(p) = &packages[i] {
            let duplicates: Vec<_> = packages[i + 1..]
                .iter()
                .positions(|it| it.as_ref().is_ok_and(|pkg| pkg.eq(p)))
                .collect();
            // remove all duplicate packages
            duplicates.into_iter().rev().for_each(|dup_pos| {
                _ = packages.remove(dup_pos + i + 1);
            });
        }
        i += 1;
    }
}

pub(crate) fn should_refresh_for_file_change(
    changed_file_path: &AbsPath,
    // change_kind: ChangeKind,
    // additional_paths: &[&str],
) -> bool {
    // const IMPLICIT_TARGET_FILES: &[&str] = &["build.rs", "src/main.rs", "src/lib.rs"];
    // const IMPLICIT_TARGET_DIRS: &[&str] = &["src/bin", "examples", "tests", "benches"];

    let changed_file_name = match changed_file_path.file_name() {
        Some(it) => it,
        None => return false,
    };

    if let "Move.toml" /*| "Cargo.lock"*/ = changed_file_name {
        return true;
    }

    // if additional_paths.contains(&file_name) {
    //     return true;
    // }

    // if change_kind == ChangeKind::Modify {
    //     return false;
    // }

    // .cargo/config{.toml}
    // if path.extension().unwrap_or_default() != "move" {
    //     let is_cargo_config = matches!(file_name, "config.toml" | "config")
    //         && path
    //             .parent()
    //             .map(|parent| parent.as_str().ends_with(".cargo"))
    //             .unwrap_or(false);
    //     return is_cargo_config;
    // }

    // if IMPLICIT_TARGET_FILES.iter().any(|it| changed_file_path.as_str().ends_with(it)) {
    //     return true;
    // }
    // let changed_file_parent = match changed_file_path.parent() {
    //     Some(it) => it,
    //     None => return false,
    // };
    // if IMPLICIT_TARGET_DIRS
    //     .iter()
    //     .any(|it| changed_file_parent.as_str().ends_with(it))
    // {
    //     return true;
    // }
    // if changed_file_name == "main.rs" {
    //     let grand_parent = match changed_file_parent.parent() {
    //         Some(it) => it,
    //         None => return false,
    //     };
    //     if IMPLICIT_TARGET_DIRS
    //         .iter()
    //         .any(|it| grand_parent.as_str().ends_with(it))
    //     {
    //         return true;
    //     }
    // }
    false
}
