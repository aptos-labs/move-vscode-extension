use crate::config::FilesWatcher;
use crate::flycheck::FlycheckHandle;
use crate::global_state::{FetchPackagesRequest, FetchPackagesResponse, GlobalState};
use crate::main_loop::Task;
use crate::op_queue::Cause;
use crate::project_folders::ProjectFolders;
use crate::{Config, lsp_ext};
use base_db::change::{FileChanges, PackageGraph};
use lsp_types::FileSystemWatcher;
use project_model::aptos_package::AptosPackage;
use std::fmt::Formatter;
use std::sync::Arc;
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
            message.push_str("Failed to load Aptos packages.");
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
        tracing::info!("schedule to the worker thread pool");
        tracing::info!(?discovered_manifests);
        self.task_pool
            .handle
            .spawn_with_sender(ThreadIntent::Worker, move |sender| {
                let _p = tracing::info_span!("on the worker thread: load packages").entered();
                sender
                    .send(Task::FetchPackagesProgress(FetchPackagesProgress::Begin))
                    .unwrap();
                let discovered_packages = discovered_manifests
                    .iter()
                    .map(|manifest_path| {
                        tracing::debug!(path = %manifest_path, "loading workspace from manifest");
                        AptosPackage::load(manifest_path)
                    })
                    .collect::<Vec<_>>();

                // figure out if it's needed or not
                // dedup(&mut discovered_packages);

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
        let switching_from_empty_workspace = self.packages.is_empty();

        tracing::info!(?force_reload_deps, %switching_from_empty_workspace);
        if self.fetch_workspace_error().is_err() && !switching_from_empty_workspace {
            if *force_reload_deps {
                self.reload_package_deps(format!("fetch workspace error while handling {:?}", cause));
            }
            // It only makes sense to switch to a partially broken workspace
            // if we don't have any workspace at all yet.
            return;
        }

        let workspaces = packages
            .iter()
            .filter_map(|res| res.as_ref().ok().cloned())
            .collect::<Vec<_>>();
        self.packages = Arc::new(workspaces);

        if let FilesWatcher::Client = self.config.files().watcher {
            let filter = self
                .packages
                .iter()
                .flat_map(|ws| ws.to_folder_roots())
                .filter(|it| it.is_local)
                .map(|it| it.include);

            let mut watchers: Vec<FileSystemWatcher> =
                if self.config.did_change_watched_files_relative_pattern_support() {
                    // When relative patterns are supported by the client, prefer using them
                    filter
                        .flat_map(|include| {
                            include.into_iter().flat_map(|base| {
                                [(base.clone(), "**/*.move"), (base.clone(), "**/Move.toml")]
                            })
                        })
                        .map(|(base, pat)| FileSystemWatcher {
                            glob_pattern: lsp_types::GlobPattern::Relative(lsp_types::RelativePattern {
                                base_uri: lsp_types::OneOf::Right(
                                    lsp_types::Url::from_file_path(base).unwrap(),
                                ),
                                pattern: pat.to_owned(),
                            }),
                            kind: None,
                        })
                        .collect()
                } else {
                    // When they're not, integrate the base to make them into absolute patterns
                    filter
                        .flat_map(|include| {
                            include.into_iter().flat_map(|base| {
                                [format!("{base}/**/*.move"), format!("{base}/**/Move.toml")]
                            })
                        })
                        .map(|glob_pattern| FileSystemWatcher {
                            glob_pattern: lsp_types::GlobPattern::String(glob_pattern),
                            kind: None,
                        })
                        .collect()
                };

            watchers.extend(
                self.packages
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

        let files_config = self.config.files();
        let project_folders = ProjectFolders::new(&self.packages, &files_config.exclude);

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

        let package_graph = {
            let mut package_graph = PackageGraph::default();
            let n_ws = self.packages.len();
            for i in 0..n_ws {
                let ws_root = self.packages.get(i).unwrap().content_root().to_string();
                {
                    self.report_progress(
                        progress_title,
                        crate::lsp::utils::Progress::Report,
                        Some(ws_root.clone()),
                        Some((i as f64) / (n_ws as f64)),
                        None,
                    );
                }

                let ws = self.packages.get(i).unwrap();
                let vfs = &self.vfs.read().0;

                let mut load = |path: &AbsPath| vfs.file_id(&vfs::VfsPath::from(path.to_path_buf()));

                let pkg_graph = ws.to_package_graph(&mut load);
                match pkg_graph {
                    Some(pkg_graph) => {
                        package_graph.extend(pkg_graph);
                    }
                    None => {
                        tracing::info!(
                            "could not load PackageGraph for package from {:?}, vfs is not ready",
                            ws_root
                        );
                    }
                }
            }
            package_graph
        };

        tracing::info!("process file changes");
        self.process_pending_file_changes();

        let mut change = FileChanges::new();
        {
            let vfs = &self.vfs.read().0;
            let roots = self.package_root_config.partition_into_roots(vfs);
            change.set_package_roots(roots);
            // depends on roots being available
            change.set_package_graph(package_graph);
        }
        self.analysis_host.apply_change(change);

        self.report_progress(progress_title, crate::lsp::utils::Progress::End, None, None, None);

        self.reload_flycheck();
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
        let config = self.config.flycheck_config(/*None*/);
        if config.is_none() {
            self.flycheck = Arc::from_iter([]);
            return;
        }

        let config = config.unwrap();
        let sender = self.flycheck_sender.clone();
        self.flycheck = self
            .packages
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
