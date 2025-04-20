use std::collections::HashMap;
use crate::config::{Config, ConfigErrors};
use crate::diagnostics::DiagnosticCollection;
use crate::flycheck::{FlycheckHandle, FlycheckMessage};
use crate::line_index::{LineEndings, LineIndex};
use crate::lsp::from_proto;
use crate::lsp::to_proto::url_from_abs_path;
use crate::main_loop::Task;
use crate::mem_docs::MemDocs;
use crate::op_queue::{Cause, OpQueue};
use crate::project_folders::PackageRootConfig;
use crate::task_pool::TaskPool;
use crate::{lsp_ext, reload};
use base_db::change::FileChange;
use crossbeam_channel::{Receiver, Sender, unbounded};
use ide::{Analysis, AnalysisHost, Cancellable};
use lang::builtin_files::BUILTINS_FILE;
use lsp_types::Url;
use parking_lot::{
    MappedRwLockReadGuard, RwLock, RwLockReadGuard, RwLockUpgradableReadGuard, RwLockWriteGuard,
};
use project_model::aptos_workspace::AptosWorkspace;
use std::sync::Arc;
use std::time::Instant;
use tracing::Level;
use vfs::{AnchoredPathBuf, FileId, VfsPath};

pub(crate) struct FetchWorkspaceRequest {
    pub(crate) force_reload_deps: bool,
}

pub(crate) struct FetchWorkspaceResponse {
    pub(crate) workspaces: Vec<anyhow::Result<AptosWorkspace>>,
    pub(crate) force_reload_deps: bool,
}

// Enforces drop order
pub(crate) struct Handle<H, C> {
    pub(crate) handle: H,
    pub(crate) receiver: C,
}

pub(crate) type ReqHandler = fn(&mut GlobalState, lsp_server::Response);
type ReqQueue = lsp_server::ReqQueue<(String, Instant), ReqHandler>;

/// `GlobalState` is the primary mutable state of the language server
///
/// The most interesting components are `vfs`, which stores a consistent
/// snapshot of the file systems, and `analysis_host`, which stores our
/// incremental salsa database.
///
/// Note that this struct has more than one impl in various modules!
pub(crate) struct GlobalState {
    sender: Sender<lsp_server::Message>,
    req_queue: ReqQueue,

    pub(crate) task_pool: Handle<TaskPool<Task>, Receiver<Task>>,

    pub(crate) config: Arc<Config>,
    pub(crate) config_errors: Option<ConfigErrors>,
    pub(crate) analysis_host: AnalysisHost,
    pub(crate) diagnostics: DiagnosticCollection,
    pub(crate) mem_docs: MemDocs,
    pub(crate) package_root_config: PackageRootConfig,

    // status
    pub(crate) shutdown_requested: bool,
    pub(crate) last_reported_status: lsp_ext::ServerStatusParams,

    // Flycheck
    pub(crate) flycheck: Arc<[FlycheckHandle]>,
    pub(crate) flycheck_sender: Sender<FlycheckMessage>,
    pub(crate) flycheck_receiver: Receiver<FlycheckMessage>,
    pub(crate) last_flycheck_error: Option<String>,

    // VFS
    pub(crate) loader: Handle<Box<dyn vfs::loader::Handle>, Receiver<vfs::loader::Message>>,
    pub(crate) vfs: Arc<RwLock<(vfs::Vfs, HashMap<FileId, LineEndings>)>>,
    pub(crate) vfs_config_version: u32,
    pub(crate) vfs_progress_config_version: u32,
    pub(crate) vfs_done: bool,
    pub(crate) builtins_file_id: FileId,
    // used to track how long VFS loading takes. this can't be on `vfs::loader::Handle`,
    // as that handle's lifetime is the same as `GlobalState` itself.
    pub(crate) vfs_span: Option<tracing::span::EnteredSpan>,
    pub(crate) wants_to_switch: Option<Cause>,

    pub(crate) workspaces: Arc<Vec<AptosWorkspace>>,
    // pub(crate) crate_graph_file_dependencies: FxHashSet<VfsPath>,

    // op queues
    pub(crate) fetch_workspaces_queue: OpQueue<FetchWorkspaceRequest, FetchWorkspaceResponse>,
}

/// An immutable snapshot of the world's state at a point in time.
pub(crate) struct GlobalStateSnapshot {
    pub(crate) config: Arc<Config>,
    pub(crate) analysis: Analysis,
    mem_docs: MemDocs,
    // pub(crate) semantic_tokens_cache: Arc<Mutex<FxHashMap<Url, SemanticTokens>>>,
    vfs: Arc<RwLock<(vfs::Vfs, HashMap<FileId, LineEndings>)>>,
    pub(crate) workspaces: Arc<Vec<AptosWorkspace>>,
    pub(crate) flycheck: Arc<[FlycheckHandle]>,
}

impl std::panic::UnwindSafe for GlobalStateSnapshot {}

impl GlobalState {
    pub(crate) fn new(sender: Sender<lsp_server::Message>, config: Config) -> GlobalState {
        let loader = {
            let (sender, receiver) = unbounded::<vfs::loader::Message>();
            let handle: vfs_notify::NotifyHandle = vfs::loader::Handle::spawn(sender);
            let handle = Box::new(handle) as Box<dyn vfs::loader::Handle>;
            Handle { handle, receiver }
        };

        let num_threads = config.main_loop_num_threads();
        let task_pool = {
            let (sender, receiver) = unbounded();
            let handle = TaskPool::new_with_threads(sender, num_threads);
            Handle { handle, receiver }
        };

        let analysis_host = AnalysisHost::new();

        let (flycheck_sender, flycheck_receiver) = unbounded();

        let vfs = Arc::new(RwLock::new((vfs::Vfs::default(), HashMap::default())));
        let builtins_file_id = {
            let vfs = &mut vfs.write().0;
            let builtins_path = VfsPath::new_virtual_path("/builtins.move".to_string());
            vfs.set_file_contents(builtins_path.clone(), Some(BUILTINS_FILE.bytes().collect()));
            vfs.file_id(&builtins_path).unwrap()
        };

        let mut this = GlobalState {
            sender,
            req_queue: ReqQueue::default(),
            task_pool,
            config: Arc::new(config.clone()),
            analysis_host,
            diagnostics: Default::default(),
            mem_docs: MemDocs::default(),
            shutdown_requested: false,
            last_reported_status: lsp_ext::ServerStatusParams {
                health: lsp_ext::Health::Ok,
                quiescent: true,
                message: None,
            },
            package_root_config: PackageRootConfig::default(),
            config_errors: Default::default(),

            flycheck: Arc::from_iter([]),
            flycheck_sender,
            flycheck_receiver,
            last_flycheck_error: None,

            loader,
            vfs,
            builtins_file_id,
            vfs_config_version: 0,
            vfs_progress_config_version: 0,
            vfs_done: true,
            vfs_span: None,
            wants_to_switch: None,

            workspaces: Arc::from(Vec::new()),
            fetch_workspaces_queue: OpQueue::default(),
        };
        // Apply any required database inputs from the config.
        this.update_configuration(config);
        this
    }

    pub(crate) fn process_file_changes(&mut self) -> bool {
        let _p = tracing::span!(Level::INFO, "GlobalState::process_changes").entered();

        let (change, refresh_workspaces) = {
            let mut change = FileChange::new();

            let mut vfs_lock = self.vfs.write();
            let changed_files = vfs_lock.0.take_changes();
            if changed_files.is_empty() {
                return false;
            }

            // downgrade to read lock to allow more readers while we are normalizing text
            let vfs_lock = RwLockWriteGuard::downgrade_to_upgradable(vfs_lock);
            let vfs: &vfs::Vfs = &vfs_lock.0;

            let mut refresh_workspaces = false;
            let mut bytes = vec![];

            for changed_file in changed_files.into_values() {
                let changed_file_vfs_path = vfs.file_path(changed_file.file_id);

                if let Some(changed_file_path) = changed_file_vfs_path.as_path() {
                    refresh_workspaces |= changed_file.is_created_or_deleted();
                    if reload::should_refresh_for_file_change(&changed_file_path) {
                        tracing::trace!(?changed_file_path, kind = ?changed_file.kind(), "refreshing for a change");
                        refresh_workspaces |= true;
                    }
                }

                // Clear native diagnostics when their file gets deleted
                if !changed_file.exists() {
                    self.diagnostics.clear_native_for(changed_file.file_id);
                }

                let text_with_line_endings =
                    if let vfs::Change::Create(v, _) | vfs::Change::Modify(v, _) = changed_file.change {
                        String::from_utf8(v).ok().map(|text| {
                            let (text, line_endings) = LineEndings::normalize(text);
                            (text, line_endings)
                        })
                    } else {
                        None
                    };

                // delay `line_endings_map` changes until we are done normalizing the text
                // this allows delaying the re-acquisition of the write lock
                bytes.push((changed_file.file_id, text_with_line_endings));
            }

            let (vfs, line_endings_map) = &mut *RwLockUpgradableReadGuard::upgrade(vfs_lock);
            bytes.into_iter().for_each(|(file_id, text_with_line_endings)| {
                let text = match text_with_line_endings {
                    None => None,
                    Some((text, line_endings)) => {
                        line_endings_map.insert(file_id, line_endings);
                        Some(text)
                    }
                };
                change.change_file(file_id, text);
            });
            if refresh_workspaces {
                let roots = self.package_root_config.partition_into_roots(vfs);
                change.set_package_roots(roots);
            }
            (change, refresh_workspaces)
        };

        let _p = tracing::span!(Level::INFO, "GlobalState::process_changes/apply_change").entered();
        self.analysis_host.apply_change(change);

        {
            if refresh_workspaces {
                let _p = tracing::span!(Level::INFO, "GlobalState::process_changes/ws_structure_change")
                    .entered();
                self.fetch_workspaces_queue.request_op(
                    "workspace vfs file change".to_string(),
                    FetchWorkspaceRequest {
                        force_reload_deps: true,
                    },
                );
            }
        }

        true
    }

    pub(crate) fn snapshot(&self) -> GlobalStateSnapshot {
        GlobalStateSnapshot {
            config: Arc::clone(&self.config),
            workspaces: Arc::clone(&self.workspaces),
            analysis: self.analysis_host.analysis(),
            vfs: Arc::clone(&self.vfs),
            // check_fixes: Arc::clone(&self.diagnostics.check_fixes),
            mem_docs: self.mem_docs.clone(),
            // semantic_tokens_cache: Arc::clone(&self.semantic_tokens_cache),
            flycheck: self.flycheck.clone(),
        }
    }

    pub(crate) fn send_request<R: lsp_types::request::Request>(
        &mut self,
        params: R::Params,
        handler: ReqHandler,
    ) {
        let request = self
            .req_queue
            .outgoing
            .register(R::METHOD.to_owned(), params, handler);
        self.send(request.into());
    }

    pub(crate) fn complete_request(&mut self, response: lsp_server::Response) {
        let handler = self
            .req_queue
            .outgoing
            .complete(response.id.clone())
            .expect("received response for unknown request");
        handler(self, response)
    }

    pub(crate) fn send_notification<N: lsp_types::notification::Notification>(&self, params: N::Params) {
        let not = lsp_server::Notification::new(N::METHOD.to_owned(), params);
        self.send(not.into());
    }

    pub(crate) fn register_request(&mut self, request: &lsp_server::Request, request_received: Instant) {
        self.req_queue
            .incoming
            .register(request.id.clone(), (request.method.clone(), request_received));
    }

    pub(crate) fn respond(&mut self, response: lsp_server::Response) {
        if let Some((method, start)) = self.req_queue.incoming.complete(&response.id) {
            if let Some(err) = &response.error {
                if err.message.starts_with("server panicked") {
                    // self.poke_rust_analyzer_developer(format!("{}, check the log", err.message))
                }
            }

            let duration = start.elapsed();
            tracing::debug!("handled {} - ({}) in {:0.2?}", method, response.id, duration);
            self.send(response.into());
        }
    }

    pub(crate) fn cancel(&mut self, request_id: lsp_server::RequestId) {
        if let Some(response) = self.req_queue.incoming.cancel(request_id) {
            self.send(response.into());
        }
    }

    pub(crate) fn is_completed(&self, request: &lsp_server::Request) -> bool {
        self.req_queue.incoming.is_completed(&request.id)
    }

    #[track_caller]
    fn send(&self, message: lsp_server::Message) {
        self.sender.send(message).unwrap();
    }

    pub(crate) fn publish_diagnostics(
        &mut self,
        uri: Url,
        version: Option<i32>,
        mut diagnostics: Vec<lsp_types::Diagnostic>,
    ) {
        // We put this on a separate thread to avoid blocking the main thread with serialization work
        self.task_pool.handle.spawn_with_sender(stdx::thread::ThreadIntent::Worker, {
            let sender = self.sender.clone();
            move |_| {
                // VSCode assumes diagnostic messages to be non-empty strings, so we need to patch
                // empty diagnostics. Neither the docs of VSCode nor the LSP spec say whether
                // diagnostic messages are actually allowed to be empty or not and patching this
                // in the VSCode client does not work as the assertion happens in the protocol
                // conversion. So this hack is here to stay, and will be considered a hack
                // until the LSP decides to state that empty messages are allowed.

                // See https://github.com/rust-lang/rust-analyzer/issues/11404
                // See https://github.com/rust-lang/rust-analyzer/issues/13130
                let patch_empty = |message: &mut String| {
                    if message.is_empty() {
                        " ".clone_into(message);
                    }
                };

                for d in &mut diagnostics {
                    patch_empty(&mut d.message);
                    if let Some(dri) = &mut d.related_information {
                        for dri in dri {
                            patch_empty(&mut dri.message);
                        }
                    }
                }

                let not = lsp_server::Notification::new(
                    <lsp_types::notification::PublishDiagnostics as lsp_types::notification::Notification>::METHOD.to_owned(),
                    lsp_types::PublishDiagnosticsParams { uri, diagnostics, version },
                );
                _ = sender.send(not.into());
            }
        });
    }
}

impl Drop for GlobalState {
    fn drop(&mut self) {
        self.analysis_host.request_cancellation();
    }
}

impl GlobalStateSnapshot {
    fn vfs_read(&self) -> MappedRwLockReadGuard<'_, vfs::Vfs> {
        RwLockReadGuard::map(self.vfs.read(), |(it, _)| it)
    }

    pub(crate) fn url_to_file_id(&self, url: &Url) -> anyhow::Result<FileId> {
        url_to_file_id(&self.vfs_read(), url)
    }

    pub(crate) fn file_id_to_url(&self, id: FileId) -> Url {
        file_id_to_url(&self.vfs_read(), id)
    }

    pub(crate) fn vfs_path_to_file_id(&self, vfs_path: &VfsPath) -> anyhow::Result<FileId> {
        vfs_path_to_file_id(&self.vfs_read(), vfs_path)
    }

    pub(crate) fn file_line_index(&self, file_id: FileId) -> Cancellable<LineIndex> {
        let endings = self.vfs.read().1[&file_id];
        let index = self.analysis.file_line_index(file_id)?;
        let res = LineIndex {
            index,
            endings,
            encoding: self.config.caps().negotiated_encoding(),
        };
        Ok(res)
    }

    pub(crate) fn file_version(&self, file_id: FileId) -> Option<i32> {
        Some(self.mem_docs.get(self.vfs_read().file_path(file_id))?.version)
    }

    pub(crate) fn url_file_version(&self, url: &Url) -> Option<i32> {
        let path = from_proto::vfs_path(url).ok()?;
        Some(self.mem_docs.get(&path)?.version)
    }

    pub(crate) fn anchored_path(&self, path: &AnchoredPathBuf) -> Url {
        let mut base = self.vfs_read().file_path(path.anchor).clone();
        base.pop();
        let path = base.join(&path.path).unwrap();
        let path = path.as_path().unwrap();
        url_from_abs_path(path)
    }

    pub(crate) fn file_id_to_file_path(&self, file_id: FileId) -> VfsPath {
        self.vfs_read().file_path(file_id).clone()
    }

    // pub(crate) fn target_spec_for_crate(&self, crate_id: CrateId) -> Option<TargetSpec> {
    //     let file_id = self.analysis.crate_root(crate_id).ok()?;
    //     let path = self.vfs_read().file_path(file_id).clone();
    //     let path = path.as_path()?;
    //
    //     for workspace in self.workspaces.iter() {
    //         match &workspace.kind {
    //             ProjectWorkspaceKind::Cargo { cargo, .. }
    //             | ProjectWorkspaceKind::DetachedFile { cargo: Some((cargo, _, _)), .. } => {
    //                 let Some(target_idx) = cargo.target_by_root(path) else {
    //                     continue;
    //                 };
    //
    //                 let target_data = &cargo[target_idx];
    //                 let package_data = &cargo[target_data.package];
    //
    //                 return Some(TargetSpec::Cargo(CargoTargetSpec {
    //                     workspace_root: cargo.workspace_root().to_path_buf(),
    //                     cargo_toml: package_data.manifest.clone(),
    //                     crate_id,
    //                     package: cargo.package_flag(package_data),
    //                     target: target_data.name.clone(),
    //                     target_kind: target_data.kind,
    //                     required_features: target_data.required_features.clone(),
    //                     features: package_data.features.keys().cloned().collect(),
    //                     sysroot_root: workspace.sysroot.root().map(ToOwned::to_owned),
    //                 }));
    //             }
    //             ProjectWorkspaceKind::Json(project) => {
    //                 let Some(krate) = project.crate_by_root(path) else {
    //                     continue;
    //                 };
    //                 let Some(build) = krate.build else {
    //                     continue;
    //                 };
    //
    //                 return Some(TargetSpec::ProjectJson(ProjectJsonTargetSpec {
    //                     label: build.label,
    //                     target_kind: build.target_kind,
    //                     shell_runnables: project.runnables().to_owned(),
    //                 }));
    //             }
    //             ProjectWorkspaceKind::DetachedFile { .. } => {}
    //         };
    //     }
    //
    //     None
    // }

    pub(crate) fn file_exists(&self, file_id: FileId) -> bool {
        self.vfs.read().0.exists(file_id)
    }
}

pub(crate) fn file_id_to_url(vfs: &vfs::Vfs, id: FileId) -> Url {
    let path = vfs.file_path(id);
    match path.as_path() {
        Some(path) => url_from_abs_path(path),
        None => {
            panic!("cannot convert builtins file {:?} into the Url", id)
        }
    }
}

pub(crate) fn url_to_file_id(vfs: &vfs::Vfs, url: &Url) -> anyhow::Result<FileId> {
    let path = from_proto::vfs_path(url)?;
    let res = vfs
        .file_id(&path)
        .ok_or_else(|| anyhow::format_err!("file not found: {path}"))?;
    Ok(res)
}

pub(crate) fn vfs_path_to_file_id(vfs: &vfs::Vfs, vfs_path: &VfsPath) -> anyhow::Result<FileId> {
    let res = vfs
        .file_id(vfs_path)
        .ok_or_else(|| anyhow::format_err!("file not found: {vfs_path}"))?;
    Ok(res)
}
