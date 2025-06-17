use crate::config::Config;
use crate::config::validation::ConfigErrors;
use crate::diagnostics::DiagnosticCollection;
use crate::flycheck::{FlycheckHandle, FlycheckMessage};
use crate::line_index::{LineEndings, LineIndex};
use crate::lsp::from_proto;
use crate::lsp::to_proto::url_from_abs_path;
use crate::lsp_ext;
use crate::lsp_ext::{MovefmtVersionError, MovefmtVersionErrorParams};
use crate::main_loop::Task;
use crate::mem_docs::MemDocs;
use crate::op_queue::{Cause, OpQueue};
use crate::task_pool::TaskPool;
use camino::Utf8PathBuf;
use crossbeam_channel::{Receiver, Sender, unbounded};
use ide::{Analysis, AnalysisHost, Cancellable};
use lang::builtins_file;
use lsp_types::Url;
use lsp_types::notification::Notification;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use project_model::aptos_package::AptosPackage;
use project_model::project_folders::PackageRootConfig;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Instant;
use syntax::files::FileRange;
use syntax::{TextRange, TextSize};
use vfs::{AnchoredPathBuf, FileId, VfsPath};

pub(crate) struct LoadPackagesRequest {
    pub(crate) force_reload_package_deps: bool,
}

pub(crate) struct LoadPackagesResponse {
    pub(crate) packages_from_fs: Vec<anyhow::Result<AptosPackage>>,
    pub(crate) force_reload_package_deps: bool,
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
    pub(crate) fmt_pool: Handle<TaskPool<Task>, Receiver<Task>>,

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
    pub(crate) flycheck_jobs: Arc<[FlycheckHandle]>,
    pub(crate) flycheck_sender: Sender<FlycheckMessage>,
    pub(crate) flycheck_receiver: Receiver<FlycheckMessage>,
    pub(crate) last_flycheck_error: Option<String>,

    // VFS
    pub(crate) vfs_loader: Handle<Box<vfs_notify::NotifyHandle>, Receiver<vfs::loader::Message>>,
    pub(crate) vfs: Arc<RwLock<(vfs::Vfs, HashMap<FileId, LineEndings>)>>,
    pub(crate) vfs_config_version: u32,
    pub(crate) vfs_progress_config_version: u32,
    pub(crate) vfs_done: bool,
    pub(crate) vfs_initialized: bool,
    // used to track how long VFS loading takes. this can't be on `vfs::loader::Handle`,
    // as that handle's lifetime is the same as `GlobalState` itself.
    pub(crate) vfs_span: Option<tracing::span::EnteredSpan>,
    pub(crate) reason_to_state_refresh: Option<Cause>,

    pub(crate) all_packages: Arc<Vec<AptosPackage>>,
    // op queues
    pub(crate) load_aptos_packages_queue: OpQueue<LoadPackagesRequest, LoadPackagesResponse>,
}

/// An immutable snapshot of the world's state at a point in time.
pub(crate) struct GlobalStateSnapshot {
    pub(crate) config: Arc<Config>,
    pub(crate) analysis: Analysis,
    mem_docs: MemDocs,
    vfs: Arc<RwLock<(vfs::Vfs, HashMap<FileId, LineEndings>)>>,
    pub(crate) all_packages: Arc<Vec<AptosPackage>>,
    pub(crate) flycheck_jobs: Arc<[FlycheckHandle]>,
    sender: Sender<lsp_server::Message>,
}

impl std::panic::UnwindSafe for GlobalStateSnapshot {}

impl GlobalState {
    pub(crate) fn new(sender: Sender<lsp_server::Message>, config: Config) -> GlobalState {
        let vfs_loader = {
            let (sender, receiver) = unbounded::<vfs::loader::Message>();
            let handle = vfs::loader::Handle::spawn(sender);
            Handle {
                handle: Box::new(handle),
                receiver,
            }
        };

        let num_threads = config.main_loop_num_threads();
        let task_pool = {
            let (sender, receiver) = unbounded();
            let handle = TaskPool::new_with_threads(sender, num_threads);
            Handle { handle, receiver }
        };
        let fmt_pool = {
            let (sender, receiver) = unbounded();
            let handle = TaskPool::new_with_threads(sender, 1);
            Handle { handle, receiver }
        };

        let (flycheck_sender, flycheck_receiver) = unbounded();

        let mut analysis_host = AnalysisHost::new();

        let vfs = Arc::new(RwLock::new((vfs::Vfs::default(), HashMap::default())));
        {
            let vfs = &mut vfs.write().0;
            let change = builtins_file::add_to_vfs(vfs);
            analysis_host.apply_change(change);
        };

        let mut this = GlobalState {
            sender,
            req_queue: ReqQueue::default(),
            task_pool,
            fmt_pool,
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

            flycheck_jobs: Arc::from_iter([]),
            flycheck_sender,
            flycheck_receiver,
            last_flycheck_error: None,

            vfs_loader,
            vfs,
            vfs_config_version: 0,
            vfs_progress_config_version: 0,
            vfs_done: true,
            vfs_initialized: false,
            vfs_span: None,
            reason_to_state_refresh: None,

            all_packages: Arc::from(Vec::new()),
            load_aptos_packages_queue: OpQueue::default(),
        };
        // Apply any required database inputs from the config.
        this.update_configuration(config);
        this
    }

    pub fn vfs_initialized_and_loaded(&self) -> bool {
        self.vfs_initialized && self.vfs_done
    }

    pub(crate) fn snapshot(&self) -> GlobalStateSnapshot {
        GlobalStateSnapshot {
            config: Arc::clone(&self.config),
            all_packages: Arc::clone(&self.all_packages),
            analysis: self.analysis_host.analysis(),
            vfs: Arc::clone(&self.vfs),
            mem_docs: self.mem_docs.clone(),
            // semantic_tokens_cache: Arc::clone(&self.semantic_tokens_cache),
            flycheck_jobs: self.flycheck_jobs.clone(),
            sender: self.sender.clone(),
        }
    }

    pub(crate) fn local_packages(&self) -> impl Iterator<Item = &AptosPackage> {
        self.all_packages.iter().filter(|it| it.is_local())
    }

    pub(crate) fn ws_root_packages(&self) -> impl Iterator<Item = &AptosPackage> {
        self.local_packages()
            .filter(|it| self.config.is_under_ws_roots(it.content_root()))
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
                    self.poke_aptos_analyzer_developer(format!("{}, check the log", err.message))
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

    #[allow(unused)]
    pub(crate) fn vfs_path_to_file_id(&self, vfs_path: &VfsPath) -> anyhow::Result<FileId> {
        vfs_path_to_file_id(&self.vfs_read(), vfs_path)
    }

    pub(crate) fn full_range(&self, file_id: FileId) -> Cancellable<FileRange> {
        let file_text = self.analysis.file_text(file_id)?;
        Ok(FileRange {
            file_id,
            range: TextRange::up_to(TextSize::of(&*file_text.deref())),
        })
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

    pub(crate) fn show_message(&self, message_type: lsp_types::MessageType, message: String) {
        let notif = lsp_server::Notification::new(
            lsp_types::notification::ShowMessage::METHOD.to_owned(),
            lsp_types::ShowMessageParams { typ: message_type, message },
        );
        self.send_notification(notif);
    }

    pub(crate) fn ask_client_for_movefmt_update(&self, message: String) {
        let mut from_env = false;
        let aptos_path = match self.config.aptos_path() {
            Some(p) => Some(p),
            None => {
                from_env = true;
                which::which("aptos")
                    .ok()
                    .and_then(|it| Utf8PathBuf::from_path_buf(it).ok())
            }
        };
        let notif = lsp_server::Notification::new(
            MovefmtVersionError::METHOD.to_owned(),
            MovefmtVersionErrorParams {
                message,
                aptos_path: aptos_path.map(|it| it.to_string()),
                aptos_path_from_PATH: from_env,
            },
        );
        self.send_notification(notif);
    }

    pub(crate) fn send_notification(&self, notif: lsp_server::Notification) {
        self.send(notif.into())
    }

    #[track_caller]
    fn send(&self, message: lsp_server::Message) {
        self.sender.send(message).unwrap();
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
    let (res, _) = vfs
        .file_id(&path)
        .ok_or_else(|| anyhow::format_err!("file not found: {path}"))?;
    Ok(res)
}

pub(crate) fn vfs_path_to_file_id(vfs: &vfs::Vfs, vfs_path: &VfsPath) -> anyhow::Result<FileId> {
    let (res, _) = vfs
        .file_id(vfs_path)
        .ok_or_else(|| anyhow::format_err!("file not found: {vfs_path}"))?;
    Ok(res)
}
