use crate::config::Config;
use crate::diagnostics::{DiagnosticsGeneration, NativeDiagnosticsFetchKind, fetch_native_diagnostics};
use crate::flycheck::FlycheckMessage;
use crate::global_state::{
    GlobalState, LoadPackagesRequest, LoadPackagesResponse, file_id_to_url, url_to_file_id,
};
use crate::handlers::dispatch::{NotificationDispatcher, RequestDispatcher};
use crate::handlers::request;
use crate::lsp::from_proto;
use crate::lsp::utils::{Progress, notification_is};
use crate::reload::FetchPackagesProgress;
use crate::{flycheck, lsp_ext};
use base_db::SourceDatabase;
use crossbeam_channel::Receiver;
use lsp_server::Connection;
use lsp_types::notification::Notification;
use paths::AbsPathBuf;
use std::fmt;
use std::ops::Div;
use std::panic::AssertUnwindSafe;
use std::time::{Duration, Instant};
use stdx::always;
use stdx::thread::ThreadIntent;
use tracing::{Level, span};
use vfs::loader::LoadingProgress;
use vfs::{FileId, VfsPath};

pub fn main_loop(config: Config, connection: Connection) -> anyhow::Result<()> {
    // Windows scheduler implements priority boosts: if thread waits for an
    // event (like a condvar), and event fires, priority of the thread is
    // temporary bumped. This optimization backfires in our case: each time the
    // `main_loop` schedules a task to run on a threadpool, the worker threads
    // gets a higher priority, and (on a machine with fewer cores) displaces the
    // main loop! We work around this by marking the main loop as a
    // higher-priority thread.
    //
    // https://docs.microsoft.com/en-us/windows/win32/procthread/scheduling-priorities
    // https://docs.microsoft.com/en-us/windows/win32/procthread/priority-boosts
    // https://github.com/rust-lang/rust-analyzer/issues/2835
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Threading::*;
        let thread = GetCurrentThread();
        let thread_priority_above_normal = 1;
        SetThreadPriority(thread, thread_priority_above_normal);
    }

    GlobalState::new(connection.sender, config).run(connection.receiver)
}

enum Event {
    Lsp(lsp_server::Message),
    Task(Task),
    Vfs(vfs::loader::Message),
    Flycheck(FlycheckMessage),
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Event::Lsp(_) => write!(f, "Event::Lsp"),
            Event::Vfs(msg) => write!(f, "Event::Vfs({msg:?})"),
            Event::Flycheck(flycheck) => write!(f, "Event::Flycheck({:?})", flycheck),
            Event::Task(task) => {
                write!(f, "Event::Task({})", task)
            }
        }
    }
}

impl fmt::Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let debug_non_verbose = |not: &lsp_server::Notification, f: &mut fmt::Formatter<'_>| {
            f.debug_struct("Notification")
                .field("method", &not.method)
                .finish()
        };

        match self {
            Event::Lsp(lsp_server::Message::Notification(not)) => {
                if notification_is::<lsp_types::notification::DidOpenTextDocument>(not)
                    || notification_is::<lsp_types::notification::DidChangeTextDocument>(not)
                {
                    return debug_non_verbose(not, f);
                }
            }
            Event::Task(Task::Response(resp)) => {
                return f
                    .debug_struct("Response")
                    .field("id", &resp.id)
                    .field("error", &resp.error)
                    .finish();
            }
            _ => (),
        }
        match self {
            Event::Lsp(it) => fmt::Debug::fmt(it, f),
            Event::Task(it) => fmt::Debug::fmt(it, f),
            Event::Vfs(it) => fmt::Debug::fmt(it, f),
            Event::Flycheck(it) => fmt::Debug::fmt(it, f),
        }
    }
}

#[derive(Debug)]
pub(crate) enum DiagnosticsTaskKind {
    Syntax(DiagnosticsGeneration, Vec<(FileId, Vec<lsp_types::Diagnostic>)>),
    Semantic(DiagnosticsGeneration, Vec<(FileId, Vec<lsp_types::Diagnostic>)>),
}

#[derive(Debug)]
pub(crate) enum Task {
    Response(lsp_server::Response),
    Retry(lsp_server::Request),
    Diagnostics(DiagnosticsTaskKind),
    FetchPackagesProgress(FetchPackagesProgress),
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Task::Response(_) => write!(f, "Task::Response"),
            Task::Retry(_) => write!(f, "Task::Retry"),
            Task::Diagnostics(kind) => match kind {
                DiagnosticsTaskKind::Syntax(..) => write!(f, "Task::Diagnostics(Syntax)"),
                DiagnosticsTaskKind::Semantic(..) => write!(f, "Task::Diagnostics(Semantic)"),
            },
            Task::FetchPackagesProgress(progress) => {
                write!(f, "Task::FetchPackagesProgress({progress})")
            }
        }
    }
}

impl GlobalState {
    pub(crate) fn run(mut self, inbox: Receiver<lsp_server::Message>) -> anyhow::Result<()> {
        tracing::info!("starting GlobalState");

        self.update_status_or_notify();

        self.load_aptos_packages_queue.request_op(
            "on startup".to_owned(),
            LoadPackagesRequest {
                force_reload_package_deps: false,
            },
        );
        if let Some((cause, LoadPackagesRequest { force_reload_package_deps })) =
            self.load_aptos_packages_queue.should_start_op()
        {
            self.load_aptos_packages_from_fs(cause, force_reload_package_deps);
        }

        while let Ok(event) = self.next_event(&inbox) {
            let Some(event) = event else {
                anyhow::bail!("client exited without proper shutdown sequence");
            };
            if matches!(
                &event,
                Event::Lsp(lsp_server::Message::Notification(lsp_server::Notification { method, .. }))
                if method == lsp_types::notification::Exit::METHOD
            ) {
                tracing::info!("received exit notification");
                return Ok(());
            }
            self.handle_event(event);
        }

        Err(anyhow::anyhow!(
            "A receiver has been dropped, something panicked!"
        ))
    }

    fn next_event(
        &self,
        inbox: &Receiver<lsp_server::Message>,
    ) -> Result<Option<Event>, crossbeam_channel::RecvError> {
        // Make sure we reply to formatting requests ASAP so the editor doesn't block
        if let Ok(task) = self.fmt_pool.receiver.try_recv() {
            return Ok(Some(Event::Task(task)));
        }

        crossbeam_channel::select! {
            recv(inbox) -> msg =>
                return Ok(msg.ok().map(Event::Lsp)),

            recv(self.task_pool.receiver) -> task =>
                task.map(Event::Task),

            recv(self.fmt_pool.receiver) -> task =>
                task.map(Event::Task),

            recv(self.vfs_loader.receiver) -> task =>
                task.map(Event::Vfs),

            recv(self.flycheck_receiver) -> task =>
                task.map(Event::Flycheck),
        }
        .map(Some)
    }

    fn handle_event(&mut self, event: Event) {
        let loop_start = Instant::now();
        let _p = tracing::info_span!("GlobalState::handle_event", %event).entered();

        let event_dbg_msg = format!("{event:?}");
        if tracing::enabled!(Level::DEBUG) {
            let task_queue_len = self.task_pool.handle.len();
            if task_queue_len > 0 {
                tracing::debug!("task queue len: {}", task_queue_len);
            }
        }

        let was_fully_loaded = self.is_projects_fully_loaded();
        match event {
            Event::Lsp(msg) => match msg {
                lsp_server::Message::Request(req) => self.on_new_request(loop_start, req),
                lsp_server::Message::Notification(not) => self.on_notification(not),
                lsp_server::Message::Response(resp) => self.complete_request(resp),
            },
            Event::Task(task) => {
                let _p = tracing::info_span!("GlobalState::handle_event/task").entered();
                self.handle_task(task);
                // Coalesce multiple task events into one loop turn
                while let Ok(task) = self.task_pool.receiver.try_recv() {
                    self.handle_task(task);
                }
            }
            Event::Vfs(message) => {
                let _p = tracing::info_span!("GlobalState::handle_event/vfs").entered();
                self.handle_vfs_msg(message);
                // Coalesce many VFS event into a single loop turn
                while let Ok(message) = self.vfs_loader.receiver.try_recv() {
                    self.handle_vfs_msg(message);
                }
            }
            Event::Flycheck(message) => {
                let _p = tracing::info_span!("GlobalState::handle_event/flycheck").entered();
                self.handle_flycheck_msg(message);
                // Coalesce many flycheck updates into a single loop turn
                while let Ok(message) = self.flycheck_receiver.try_recv() {
                    self.handle_flycheck_msg(message);
                }
            }
        }
        let event_handling_duration = loop_start.elapsed();

        let (any_file_changed, memdocs_added_or_removed) = if self.vfs_done {
            if let Some(refresh_cause) = self.reason_to_state_refresh.take() {
                self.switch_packages(refresh_cause);
            }
            (self.process_pending_file_changes(), self.mem_docs.take_changes())
        } else {
            (false, false)
        };

        if self.is_projects_fully_loaded() {
            let became_fully_loaded = !was_fully_loaded;
            if became_fully_loaded {
                if self.config.check_on_save() {
                    // Project has loaded properly, kick off initial flycheck
                    self.flycheck_jobs.iter().for_each(|flycheck| flycheck.restart());
                }
            }

            let ask_for_client_refresh = became_fully_loaded || any_file_changed;
            if ask_for_client_refresh {
                // Refresh semantic tokens if the client supports it.
                if self.config.semantic_tokens_refresh() {
                    // self.semantic_tokens_cache.lock().clear();
                    self.send_request::<lsp_types::request::SemanticTokensRefresh>((), |_, _| ());
                }

                // Refresh code lens if the client supports it.
                if self.config.code_lens_refresh() {
                    self.send_request::<lsp_types::request::CodeLensRefresh>((), |_, _| ());
                }

                // Refresh inlay hints if the client supports it.
                if self.config.inlay_hints_refresh() {
                    self.send_request::<lsp_types::request::InlayHintRefreshRequest>((), |_, _| ());
                }

                if self.config.diagnostics_refresh() {
                    self.send_request::<lsp_types::request::WorkspaceDiagnosticRefresh>((), |_, _| ());
                }
            }

            let project_or_mem_docs_changed =
                became_fully_loaded || any_file_changed || memdocs_added_or_removed;
            if project_or_mem_docs_changed
                && !self.config.text_document_diagnostic_pull_enabled()
                && self.config.diagnostics_enabled()
            {
                self.update_diagnostics();
            }
        }

        if let Some(diagnostic_changes) = self.diagnostics.take_changes() {
            for file_id in diagnostic_changes {
                let uri = { file_id_to_url(&self.vfs.read().0, file_id) };
                let version = from_proto::vfs_path(&uri)
                    .ok()
                    .and_then(|path| self.mem_docs.get(&path).map(|it| it.version));

                let diagnostics = self
                    .diagnostics
                    .diagnostics_for(file_id)
                    .cloned()
                    .collect::<Vec<_>>();
                self.publish_diagnostics(uri, version, diagnostics);
            }
        }

        if self.config.autorefresh_on_move_toml_changes() {
            if let Some((cause, LoadPackagesRequest { force_reload_package_deps })) =
                self.load_aptos_packages_queue.should_start_op()
            {
                self.load_aptos_packages_from_fs(cause, force_reload_package_deps);
            }
        }

        self.update_status_or_notify();

        let loop_duration = loop_start.elapsed();
        if loop_duration > Duration::from_millis(100) && was_fully_loaded {
            tracing::warn!(
                "overly long loop turn took {loop_duration:?} (event handling took {event_handling_duration:?}): {event_dbg_msg}"
            );
            self.poke_aptos_analyzer_developer(format!(
                "overly long loop turn took {loop_duration:?} (event handling took {event_handling_duration:?}): {event_dbg_msg}"
            ));
        }
    }

    fn update_diagnostics(&mut self) {
        let db = self.analysis_host.raw_database();
        let generation = self.diagnostics.next_generation();
        let subscriptions = {
            let vfs = &self.vfs.read().0;
            self.mem_docs
                .iter()
                .map(|path| vfs.file_id(path).unwrap().0)
                .filter(|&file_id| {
                    let package_id = db.file_package_id(file_id);
                    // Only publish diagnostics for files in the workspace, not from crates.io deps
                    // or the sysroot.
                    // While theoretically these should never have errors, we have quite a few false
                    // positives particularly in the stdlib, and those diagnostics would stay around
                    // forever if we emitted them here.
                    !db.package_root(package_id).data(db).is_library()
                })
                .collect::<std::sync::Arc<_>>()
        };
        tracing::trace!("updating notifications for {:?}", subscriptions);
        // Split up the work on multiple threads, but we don't wanna fill the entire task pool with
        // diagnostic tasks, so we limit the number of tasks to a quarter of the total thread pool.
        let max_tasks = self.config.main_loop_num_threads().div(4).max(1);
        let chunk_length = subscriptions.len() / max_tasks;
        let remainder = subscriptions.len() % max_tasks;

        let mut start = 0;
        for task_idx in 0..max_tasks {
            let extra = if task_idx < remainder { 1 } else { 0 };
            let end = start + chunk_length + extra;
            let slice = start..end;
            if slice.is_empty() {
                break;
            }
            // Diagnostics are triggered by the user typing
            // so we run them on a latency sensitive thread.
            let snapshot = self.snapshot();
            self.task_pool
                .handle
                .spawn_with_sender(ThreadIntent::LatencySensitive, {
                    let subscriptions = subscriptions.clone();
                    // Do not fetch semantic diagnostics (and populate query results) if we haven't even
                    // loaded the initial workspace yet.
                    let fetch_semantic =
                        self.vfs_done && self.load_aptos_packages_queue.last_op_result().is_some();
                    move |sender| {
                        // We aren't observing the semantics token cache here
                        let snapshot = AssertUnwindSafe(&snapshot);
                        let Ok(diags) = std::panic::catch_unwind(|| {
                            fetch_native_diagnostics(
                                &snapshot,
                                subscriptions.clone(),
                                slice.clone(),
                                NativeDiagnosticsFetchKind::Syntax,
                            )
                        }) else {
                            return;
                        };
                        sender
                            .send(Task::Diagnostics(DiagnosticsTaskKind::Syntax(generation, diags)))
                            .unwrap();

                        if fetch_semantic {
                            let Ok(sema_diags) = std::panic::catch_unwind(|| {
                                fetch_native_diagnostics(
                                    &snapshot,
                                    subscriptions.clone(),
                                    slice.clone(),
                                    NativeDiagnosticsFetchKind::Semantic,
                                )
                            }) else {
                                return;
                            };
                            sender
                                .send(Task::Diagnostics(DiagnosticsTaskKind::Semantic(
                                    generation, sema_diags,
                                )))
                                .unwrap();
                        }
                    }
                });
            start = end;
        }
    }

    fn update_status_or_notify(&mut self) {
        let status = self.current_status();
        if self.last_reported_status != status {
            self.last_reported_status = status.clone();

            if self.config.server_status_notification() {
                self.send_notification::<lsp_ext::ServerStatusNotification>(status);
            } else if let (health @ (lsp_ext::Health::Warning | lsp_ext::Health::Error), Some(message)) =
                (status.health, &status.message)
            {
                let open_log_button =
                    tracing::enabled!(Level::ERROR) && self.load_packages_error().is_err();
                self.show_message(
                    match health {
                        lsp_ext::Health::Ok => lsp_types::MessageType::INFO,
                        lsp_ext::Health::Warning => lsp_types::MessageType::WARNING,
                        lsp_ext::Health::Error => lsp_types::MessageType::ERROR,
                    },
                    message.clone(),
                    open_log_button,
                );
            }
        }
    }

    fn handle_task(&mut self, task: Task) {
        let _p = tracing::info_span!("GlobalState::handle_task", task = %task).entered();
        match task {
            Task::Response(response) => self.respond(response),
            // Only retry requests that haven't been cancelled. Otherwise we do unnecessary work.
            Task::Retry(req) if !self.is_completed(&req) => self.on_request(req),
            Task::Retry(_) => (),
            Task::Diagnostics(kind) => {
                self.diagnostics.set_native_diagnostics(kind);
            }
            Task::FetchPackagesProgress(progress) => {
                let (state, msg) = match progress {
                    FetchPackagesProgress::Begin => (Progress::Begin, None),
                    FetchPackagesProgress::Report(msg) => (Progress::Report, Some(msg)),
                    FetchPackagesProgress::End(packages_from_fs, force_reload_package_deps) => {
                        self.load_aptos_packages_queue.op_completed(LoadPackagesResponse {
                            packages_from_fs,
                            force_reload_package_deps,
                        });
                        if let Err(fetch_err) = self.load_packages_error() {
                            tracing::error!("FetchWorkspaceError: {fetch_err}");
                        }
                        self.reason_to_state_refresh = Some("loaded aptos packages from fs".to_owned());
                        self.diagnostics.clear_flycheck_all();
                        (Progress::End, None)
                    }
                };

                self.report_progress("Fetching", state, msg, None, None);
            }
        }
    }

    fn handle_vfs_msg(&mut self, message: vfs::loader::Message) {
        let _p = tracing::info_span!("GlobalState::handle_vfs_msg").entered();
        match message {
            vfs::loader::Message::Loaded { files } => {
                let _p = tracing::info_span!("GlobalState::handle_vfs_msg{loaded}").entered();
                self.load_files_into_vfs(files, false);
            }
            vfs::loader::Message::Changed { files } => {
                let _p = tracing::info_span!("GlobalState::handle_vfs_msg{changed}").entered();
                self.load_files_into_vfs(files, true);
            }
            vfs::loader::Message::Progress {
                n_total,
                n_done,
                dir,
                config_version,
            } => {
                let _p = span!(Level::INFO, "GlobalState::handle_vfs_msg/progress").entered();
                always!(config_version <= self.vfs_config_version);

                let (n_done, state) = match n_done {
                    LoadingProgress::Started => {
                        self.vfs_span = Some(span!(Level::INFO, "vfs_load", total = n_total).entered());
                        (0, Progress::Begin)
                    }
                    LoadingProgress::Progress(n_done) => (n_done.min(n_total), Progress::Report),
                    LoadingProgress::Finished => {
                        self.vfs_span = None;
                        (n_total, Progress::End)
                    }
                };
                self.vfs_progress_config_version = config_version;

                let is_vfs_load_ended = state == Progress::End;

                if !self.vfs_initialized && is_vfs_load_ended {
                    self.vfs_initialized = true;
                }
                self.vfs_done = is_vfs_load_ended;

                if is_vfs_load_ended {
                    self.recreate_package_graph("after vfs_refresh".to_string() /*, false*/);
                }

                let mut message = format!("{n_done}/{n_total}");
                if let Some(dir) = dir {
                    message += &format!(
                        ": {}",
                        match dir.strip_prefix(self.config.root_path()) {
                            Some(relative_path) => relative_path.as_utf8_path(),
                            None => dir.as_ref(),
                        }
                    );
                }

                self.report_progress(
                    "Roots Scanned",
                    state,
                    Some(message),
                    Some(Progress::fraction(n_done, n_total)),
                    None,
                );
            }
        }
    }

    fn load_files_into_vfs(&mut self, files: Vec<(AbsPathBuf, Option<Vec<u8>>)>, is_changed: bool) {
        tracing::debug!("load files into vfs, n_files = {}", files.len());
        let vfs = &mut self.vfs.write().0;
        for (path, contents) in files {
            let path = VfsPath::from(path);
            // if the file is in mem docs, it's managed by the client via notifications
            // so only set it if its not in there
            if !self.mem_docs.contains(&path) && (is_changed || vfs.file_id(&path).is_none()) {
                vfs.set_file_contents(path, contents);
            }
        }
    }

    fn handle_flycheck_msg(&mut self, message: FlycheckMessage) {
        let _p = tracing::span!(Level::INFO, "flycheck ", ?message).entered();
        match message {
            FlycheckMessage::AddDiagnostic { ws_id, diagnostic } => {
                let snap = self.snapshot();
                let mapped_diagnostic =
                    crate::diagnostics::to_proto::map_aptos_diagnostic_to_lsp(&diagnostic, &snap);
                match mapped_diagnostic {
                    Ok(diag) => match url_to_file_id(&self.vfs.read().0, &diag.url) {
                        Ok(file_id) => {
                            self.diagnostics
                                .add_flycheck_diagnostic(ws_id, file_id, diag.diagnostic)
                        }
                        Err(err) => {
                            tracing::error!(
                                "flycheck {ws_id}: File with cargo diagnostic not found in VFS: {}",
                                err
                            );
                        }
                    },
                    Err(err) => {
                        tracing::error!(
                            "flycheck {ws_id}: Error mapping diagnostic to lsp types: {}",
                            err
                        );
                    }
                }
            }

            FlycheckMessage::ClearDiagnostics { ws_id } => self.diagnostics.clear_flycheck(ws_id),

            FlycheckMessage::Progress { ws_id: id, progress } => {
                let (state, message) = match progress {
                    flycheck::Progress::DidStart => (Progress::Begin, None),
                    flycheck::Progress::DidCancel => {
                        self.last_flycheck_error = None;
                        (Progress::End, None)
                    }
                    flycheck::Progress::DidFailToRestart(err) => {
                        self.last_flycheck_error = Some(format!("cargo check failed to start: {err}"));
                        return;
                    }
                    flycheck::Progress::DidFinish(result) => {
                        self.last_flycheck_error = result
                            .err()
                            .map(|err| format!("cargo check failed to start: {err}"));
                        (Progress::End, None)
                    }
                };

                let flycheck_config = self.config.flycheck_config().unwrap();

                // When we're running multiple flychecks, we have to include a disambiguator in
                // the title, or the editor complains. Note that this is a user-facing string.
                let title = if self.flycheck_jobs.len() == 1 {
                    format!("{}", flycheck_config)
                } else {
                    format!("{} (#{})", flycheck_config, id + 1)
                };
                self.report_progress(
                    &title,
                    state,
                    message,
                    None,
                    Some(format!("aptos-analyzer/flycheck/{id}")),
                );
            }
        }
    }

    /// Registers and handles a request. This should only be called once per incoming request.
    fn on_new_request(&mut self, request_received: Instant, req: lsp_server::Request) {
        let _p = span!(Level::INFO, "GlobalState::on_new_request", req.method = ?req.method).entered();
        self.register_request(&req, request_received);
        self.on_request(req);
    }

    /// Handles a request.
    fn on_request(&mut self, req: lsp_server::Request) {
        let mut dispatcher = RequestDispatcher {
            req: Some(req),
            global_state: self,
        };
        dispatcher.on_sync_mut::<lsp_types::request::Shutdown>(|s, ()| {
            s.shutdown_requested = true;
            Ok(())
        });

        match &mut dispatcher {
            RequestDispatcher {
                req: Some(req),
                global_state: this,
            } if this.shutdown_requested => {
                this.respond(lsp_server::Response::new_err(
                    req.id.clone(),
                    lsp_server::ErrorCode::InvalidRequest as i32,
                    "Shutdown already requested.".to_owned(),
                ));
                return;
            }
            _ => (),
        }

        use crate::handlers::request as handlers;
        use lsp_types::request as lsp_request;

        const RETRY: bool = true;
        const NO_RETRY: bool = false;

        #[rustfmt::skip]
        dispatcher
            // Request handlers that must run on the main thread
            // because they mutate GlobalState:
            // .on_sync_mut::<lsp_ext::ReloadWorkspace>(handlers::handle_workspace_reload)
            // .on_sync_mut::<lsp_ext::MemoryUsage>(handlers::handle_memory_usage)
            // .on_sync_mut::<lsp_ext::RunTest>(handlers::handle_run_test)
            // Request handlers which are related to the user typing
            // are run on the main thread to reduce latency:
            // .on_sync::<lsp_ext::JoinLines>(handlers::handle_join_lines)
            // .on_sync::<lsp_ext::OnEnter>(handlers::handle_on_enter)
            .on_sync::<lsp_request::SelectionRangeRequest>(handlers::handle_selection_range)
            // .on_sync::<lsp_ext::MatchingBrace>(handlers::handle_matching_brace)
            // .on_sync::<lsp_ext::OnTypeFormatting>(handlers::handle_on_type_formatting)
            // Formatting should be done immediately as the editor might wait on it, but we can't
            // put it on the main thread as we do not want the main thread to block on movefmt.
            // So we have an extra thread just for formatting requests to make sure it gets handled
            // as fast as possible.
            .on_fmt_thread::<lsp_request::Formatting>(handlers::handle_formatting)
            // .on_fmt_thread::<lsp_request::RangeFormatting>(handlers::handle_range_formatting)
            // We can’t run latency-sensitive request handlers which do semantic
            // analysis on the main thread because that would block other
            // requests. Instead, we run these request handlers on higher priority
            // threads in the threadpool.
            // FIXME: Retrying can make the result of this stale?
            .on_latency_sensitive::<RETRY, lsp_request::Completion>(handlers::handle_completion)
            // FIXME: Retrying can make the result of this stale
            // .on_latency_sensitive::<RETRY, lsp_request::ResolveCompletionItem>(handlers::handle_completion_resolve)
            .on_latency_sensitive::<RETRY, lsp_request::SemanticTokensFullRequest>(handlers::handle_semantic_tokens_full)
            // .on_latency_sensitive::<RETRY, lsp_request::SemanticTokensFullDeltaRequest>(handlers::handle_semantic_tokens_full_delta)
            .on_latency_sensitive::<NO_RETRY, lsp_request::SemanticTokensRangeRequest>(handlers::handle_semantic_tokens_range)
            // FIXME: Some of these NO_RETRY could be retries if the file they are interested didn't change.
            // All other request handlers
            .on_with_vfs_default::<lsp_request::DocumentDiagnosticRequest>(
                handlers::handle_document_diagnostics, || lsp_types::DocumentDiagnosticReportResult::Report(
                    lsp_types::DocumentDiagnosticReport::Full(
                        lsp_types::RelatedFullDocumentDiagnosticReport {
                            related_documents: None,
                            full_document_diagnostic_report: lsp_types::FullDocumentDiagnosticReport {
                                result_id: Some("aptos-analyzer".to_owned()),
                                items: vec![],
                            },
                        },
                    ),
            ), || lsp_server::ResponseError {
                code: lsp_server::ErrorCode::ServerCancelled as i32,
                message: "server cancelled the request".to_owned(),
                data: serde_json::to_value(lsp_types::DiagnosticServerCancellationData {
                    retrigger_request: true
                }).ok(),
            })
            .on::<RETRY, lsp_request::DocumentSymbolRequest>(handlers::handle_document_symbol)
            // .on::<RETRY, lsp_request::FoldingRangeRequest>(handlers::handle_folding_range)
            // .on::<NO_RETRY, lsp_request::SignatureHelpRequest>(handlers::handle_signature_help)
            // .on::<RETRY, lsp_request::WillRenameFiles>(handlers::handle_will_rename_files)
            .on::<NO_RETRY, lsp_request::GotoDefinition>(handlers::handle_goto_definition)
            // .on::<NO_RETRY, lsp_request::GotoDeclaration>(handlers::handle_goto_declaration)
            // .on::<NO_RETRY, lsp_request::GotoImplementation>(handlers::handle_goto_implementation)
            // .on::<NO_RETRY, lsp_request::GotoTypeDefinition>(handlers::handle_goto_type_definition)
            .on::<NO_RETRY, lsp_request::InlayHintRequest>(handlers::handle_inlay_hints)
            .on_identity::<NO_RETRY, lsp_request::InlayHintResolveRequest, _>(handlers::handle_inlay_hints_resolve)
            // .on::<NO_RETRY, lsp_request::CodeLensRequest>(handlers::handle_code_lens)
            // .on_identity::<NO_RETRY, lsp_request::CodeLensResolve, _>(handlers::handle_code_lens_resolve)
            .on::<NO_RETRY, lsp_request::PrepareRenameRequest>(handlers::handle_prepare_rename)
            .on::<NO_RETRY, lsp_request::Rename>(handlers::handle_rename)
            .on::<NO_RETRY, lsp_request::References>(handlers::handle_references)
            .on::<NO_RETRY, lsp_request::DocumentHighlightRequest>(handlers::handle_document_highlight)
            // .on::<NO_RETRY, lsp_request::CallHierarchyPrepare>(handlers::handle_call_hierarchy_prepare)
            // .on::<NO_RETRY, lsp_request::CallHierarchyIncomingCalls>(handlers::handle_call_hierarchy_incoming)
            // .on::<NO_RETRY, lsp_request::CallHierarchyOutgoingCalls>(handlers::handle_call_hierarchy_outgoing)
            // All other request handlers (lsp extension)
            .on::<RETRY, lsp_ext::AnalyzerStatus>(handlers::handle_analyzer_status)
            .on::<RETRY, lsp_request::WorkspaceSymbolRequest>(handlers::handle_workspace_symbol)
            .on::<NO_RETRY, lsp_ext::ViewSyntaxTree>(request::handle_view_syntax_tree)
            // .on::<NO_RETRY, lsp_ext::RelatedTests>(handlers::handle_related_tests)
            .on::<NO_RETRY, lsp_request::CodeActionRequest>(request::handle_code_action)
            .on_identity::<RETRY, lsp_request::CodeActionResolveRequest, _>(request::handle_code_action_resolve)
            .on::<NO_RETRY, lsp_request::HoverRequest>(handlers::handle_hover)
            .finish();
    }

    /// Handles an incoming notification.
    fn on_notification(&mut self, not: lsp_server::Notification) {
        let _p = span!(Level::INFO, "GlobalState::on_notification", not.method = ?not.method).entered();
        use crate::handlers::notification as handlers;
        use lsp_types::notification;

        NotificationDispatcher {
            not: Some(not),
            global_state: self,
        }
        .on_sync_mut::<notification::Cancel>(handlers::handle_cancel)
        .on_sync_mut::<notification::WorkDoneProgressCancel>(handlers::handle_work_done_progress_cancel)
        .on_sync_mut::<notification::DidOpenTextDocument>(handlers::handle_did_open_text_document)
        .on_sync_mut::<notification::DidChangeTextDocument>(handlers::handle_did_change_text_document)
        .on_sync_mut::<notification::DidCloseTextDocument>(handlers::handle_did_close_text_document)
        .on_sync_mut::<notification::DidSaveTextDocument>(handlers::handle_did_save_text_document)
        .on_sync_mut::<notification::DidChangeConfiguration>(handlers::handle_did_change_configuration)
        .on_sync_mut::<notification::DidChangeWorkspaceFolders>(
            handlers::handle_did_change_workspace_folders,
        )
        .on_sync_mut::<notification::DidChangeWatchedFiles>(handlers::handle_did_change_watched_files)
        .on_sync_mut::<lsp_ext::RunFlycheck>(handlers::handle_run_flycheck)
        .on_sync_mut::<lsp_ext::CancelFlycheck>(handlers::handle_cancel_flycheck)
        .on_sync_mut::<lsp_ext::ClearFlycheck>(handlers::handle_clear_flycheck)
        .finish();
    }
}
