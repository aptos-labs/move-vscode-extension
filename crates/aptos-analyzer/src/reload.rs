use crate::config::FilesWatcher;
use crate::flycheck::FlycheckHandle;
use crate::global_state::{FetchPackagesRequest, FetchPackagesResponse, GlobalState};
use crate::lsp::utils::Progress;
use crate::main_loop::Task;
use crate::op_queue::Cause;
use crate::{Config, lsp_ext};
use base_db::change::FileChanges;
use lsp_types::FileSystemWatcher;
use project_model::aptos_package::{AptosPackage, load_from_fs};
use project_model::dep_graph::collect;
use project_model::project_folders::ProjectFolders;
use std::fmt::Formatter;
use std::sync::Arc;
use std::{fmt, mem};
use stdx::format_to;
use stdx::thread::ThreadIntent;
use vfs::AbsPath;
use vfs::loader::Handle;

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
            && !self.fetch_packages_from_fs_queue.op_in_progress()
            && self.vfs_progress_config_version >= self.vfs_config_version
    }

    // /// Is the server ready to respond to analysis dependent LSP requests?
    // ///
    // /// Unlike `is_quiescent`, this returns false when we're indexing
    // /// the project, because we're holding the salsa lock and cannot
    // /// respond to LSP requests that depend on salsa data.
    // fn is_fully_ready(&self) -> bool {
    //     self.is_quiescent() /* && !self.prime_caches_queue.op_in_progress()*/
    // }

    pub(crate) fn update_configuration(&mut self, config: Config) {
        let _p = tracing::info_span!("GlobalState::update_configuration").entered();
        let old_config = mem::replace(&mut self.config, Arc::new(config));

        if self.config.discovered_manifests() != old_config.discovered_manifests() {
            let req = FetchPackagesRequest { force_reload_deps: false };
            self.fetch_packages_from_fs_queue
                .request_op("discovered projects changed".to_owned(), req)
        } else if self.config.flycheck_config() != old_config.flycheck_config() {
            self.reload_flycheck();
        }
    }

    pub(crate) fn current_status(&self) -> lsp_ext::ServerStatusParams {
        let mut status = lsp_ext::ServerStatusParams {
            health: lsp_ext::Health::Ok,
            quiescent: self.is_quiescent(),
            message: None,
        };
        let mut message = String::new();

        if !self.config.cargo_autoreload_config()
            && self.is_quiescent()
            && self.fetch_packages_from_fs_queue.op_requested()
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
        if self.fetch_packages_error().is_err() {
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
    pub(crate) fn fetch_packages_from_fs(&mut self, _cause: Cause, force_reload_deps: bool) {
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
                // hits the filesystem directly
                let fetched_packages = load_from_fs::load_aptos_packages(discovered_manifests);
                sender
                    .send(Task::FetchPackagesProgress(FetchPackagesProgress::End(
                        fetched_packages,
                        force_reload_deps,
                    )))
                    .unwrap();
            });
    }

    #[tracing::instrument(level = "info", skip(self))]
    pub(crate) fn switch_packages(&mut self, cause: Cause) {
        let Some(FetchPackagesResponse {
            packages_from_fs,
            force_reload_deps,
        }) = self.fetch_packages_from_fs_queue.last_op_result()
        else {
            return;
        };

        let switching_from_empty_workspace = self.all_packages.is_empty();
        tracing::info!(%switching_from_empty_workspace, %force_reload_deps);

        if let Err(fetch_packages_error) = self.fetch_packages_error() {
            tracing::info!(%fetch_packages_error);
            // already have a workspace, let's keep it instead of loading invalid state
            if !switching_from_empty_workspace {
                if *force_reload_deps {
                    self.recreate_package_graph(
                        format!("fetch packages error while handling {:?}", cause),
                        switching_from_empty_workspace,
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

        let same_packages = packages_from_fs.len() == self.all_packages.len()
            && packages_from_fs
                .iter()
                .zip(self.all_packages.iter())
                .all(|(l, r)| l.eq(r));

        if same_packages {
            if switching_from_empty_workspace {
                // Switching from empty to empty is a no-op
                return;
            }
            tracing::info!(?force_reload_deps, "packages are unchanged");
            if *force_reload_deps {
                self.recreate_package_graph(cause, switching_from_empty_workspace);
            }
            // Unchanged workspaces, nothing to do here
            return;
        }

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

        self.all_packages = Arc::new(packages_from_fs);
        self.package_root_config = project_folders.package_root_config;

        self.recreate_package_graph(cause, switching_from_empty_workspace);

        tracing::info!("did switch workspaces");
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
    fn recreate_package_graph(&mut self, cause: String, initial_build: bool) {
        let progress_title = "Reloading Aptos packages";
        self.report_progress(
            progress_title,
            Progress::Begin,
            Some(format!("after {:?}", cause)),
            None,
            None,
        );
        let package_graph = {
            if initial_build {
                // with initial build, vfs might not be ready, so we need to load files manually
                let vfs = &mut self.vfs.write().0;
                let mut load = |path: &AbsPath| {
                    let contents = self.vfs_loader.handle.load_sync(path);
                    let path = vfs::VfsPath::from(path.to_path_buf());
                    vfs.set_file_contents(path.clone(), contents);
                    vfs.file_id(&path).and_then(|(file_id, excluded)| {
                        (excluded == vfs::FileExcluded::No).then_some(file_id)
                    })
                };
                collect(self.all_packages.as_slice(), &mut load)
            } else {
                let vfs = &self.vfs.read().0;
                let mut load = |path: &AbsPath| {
                    tracing::debug!(?path, "load from vfs");
                    vfs.file_id(&vfs::VfsPath::from(path.to_path_buf()))
                        .map(|it| it.0)
                };
                collect(self.all_packages.as_slice(), &mut load)
            }
        };

        self.report_progress(progress_title, Progress::End, None, None, None);
        let Some(package_graph) = package_graph else {
            tracing::info!("cannot reload package dep graph, vfs is not ready yet");
            return;
        };

        let mut change = FileChanges::new();
        change.set_package_roots(
            self.package_root_config
                .partition_into_package_roots(&self.vfs.read().0),
        );
        // depends on roots being available
        change.set_package_graph(package_graph);

        self.analysis_host.apply_change(change);

        self.process_pending_file_changes();
        self.reload_flycheck();
    }

    pub(super) fn fetch_packages_error(&self) -> Result<(), String> {
        let mut buf = String::new();

        let Some(FetchPackagesResponse { packages_from_fs, .. }) =
            self.fetch_packages_from_fs_queue.last_op_result()
        else {
            return Ok(());
        };

        if packages_from_fs.is_empty() {
            format_to!(buf, "aptos-analyzer failed to find any packages");
        } else {
            for package_from_fs in packages_from_fs {
                if let Err(load_err) = package_from_fs {
                    format_to!(buf, "aptos-analyzer failed to load package: {:#}\n", load_err);
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
            .ws_root_packages()
            .enumerate()
            .filter_map(|(id, ws)| Some((id, ws.content_root(), ws.manifest_path())))
            .map(|(ws_id, ws_root, _)| {
                FlycheckHandle::spawn(ws_id, sender.clone(), config.clone(), ws_root.to_path_buf())
            })
            .collect::<Vec<_>>()
            .into();
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
