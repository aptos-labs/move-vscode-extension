use crate::command::CommandHandle;
use crate::toolchain;
use camino::Utf8PathBuf;
use crossbeam_channel::{Receiver, Sender, select_biased, unbounded};
use paths::AbsPathBuf;
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use std::{fmt, io};

pub(crate) mod compiler_diagnostic;
pub use crate::flycheck::compiler_diagnostic::AptosDiagnostic;

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub(crate) struct AptosCliOptions {
    pub(crate) extra_args: Vec<String>,
    pub(crate) extra_env: HashMap<String, String>,
}

impl AptosCliOptions {
    pub(crate) fn apply_on_command(&self, cmd: &mut Command) {
        cmd.envs(&self.extra_env);
        cmd.args(&self.extra_args);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FlycheckConfig {
    pub(crate) enabled: bool,
    command: String,
    aptos_cli: Utf8PathBuf,
    options: AptosCliOptions,
}

impl FlycheckConfig {
    pub fn new(enabled: bool, aptos_cli: Utf8PathBuf, command: &str, options: AptosCliOptions) -> Self {
        FlycheckConfig {
            enabled,
            aptos_cli,
            command: command.to_string(),
            options,
        }
    }

    pub fn command(&self) -> String {
        self.command.clone()
    }
}

impl fmt::Display for FlycheckConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "aptos move {}", self.command)
    }
}

/// Flycheck wraps the shared state and communication machinery used for
/// running `aptos move compile` (or other compatible command) and providing
/// diagnostics based on the output.
/// The spawned thread is shut down when this struct is dropped.
#[derive(Debug)]
pub(crate) struct FlycheckHandle {
    // XXX: drop order is significant
    sender: Sender<StateChange>,
    _thread: stdx::thread::JoinHandle,
    ws_id: usize,
}

impl FlycheckHandle {
    pub(crate) fn spawn(
        ws_id: usize,
        sender: Sender<FlycheckMessage>,
        config: FlycheckConfig,
        workspace_root: AbsPathBuf,
    ) -> FlycheckHandle {
        let actor = FlycheckActor::new(ws_id, sender, config, workspace_root);
        let (sender, receiver) = unbounded::<StateChange>();
        let thread = stdx::thread::Builder::new(stdx::thread::ThreadIntent::Worker, "Flycheck")
            .spawn(move || actor.run(receiver))
            .expect("failed to spawn thread");
        FlycheckHandle {
            ws_id,
            sender,
            _thread: thread,
        }
    }

    pub(crate) fn restart(&self) {
        self.sender.send(StateChange::Restart).unwrap();
    }

    /// Stop this `aptos move compile` worker.
    pub(crate) fn cancel(&self) {
        self.sender.send(StateChange::Cancel).unwrap();
    }

    pub(crate) fn ws_id(&self) -> usize {
        self.ws_id
    }
}

pub(crate) enum FlycheckMessage {
    /// Request adding a diagnostic with fixes included to a file
    AddDiagnostic { ws_id: usize, diagnostic: AptosDiagnostic },

    /// Request clearing all outdated diagnostics.
    ClearDiagnostics { ws_id: usize },

    /// Request check progress notification to client
    Progress {
        /// Flycheck instance ID
        ws_id: usize,
        progress: Progress,
    },
}

impl fmt::Debug for FlycheckMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlycheckMessage::AddDiagnostic { ws_id, diagnostic } => f
                .debug_struct("AddDiagnostic")
                .field("ws_id", ws_id)
                .field("diagnostic_code", &diagnostic.code.as_ref())
                .finish(),
            FlycheckMessage::ClearDiagnostics { ws_id } => {
                f.debug_struct("ClearDiagnostics").field("ws_id", ws_id).finish()
            }
            FlycheckMessage::Progress { ws_id, progress } => f
                .debug_struct("Progress")
                .field("ws_id", ws_id)
                .field("progress", progress)
                .finish(),
        }
    }
}

#[derive(Debug)]
pub(crate) enum Progress {
    DidStart,
    DidFinish(io::Result<()>),
    DidCancel,
    DidFailToRestart(String),
}

enum StateChange {
    Restart,
    Cancel,
}

/// A [`FlycheckActor`] is a single check instance of a workspace.
struct FlycheckActor {
    /// The workspace id of this flycheck instance.
    ws_id: usize,

    sender: Sender<FlycheckMessage>,
    config: FlycheckConfig,
    /// Either the workspace root of the workspace we are flychecking,
    /// or the project root of the project.
    root: Arc<AbsPathBuf>,

    /// CargoHandle exists to wrap around the communication needed to be able to
    /// run `cargo check` without blocking. Currently the Rust standard library
    /// doesn't provide a way to read sub-process output without blocking, so we
    /// have to wrap sub-processes output handling in a thread and pass messages
    /// back over a channel.
    command_handle: Option<CommandHandle<AptosDiagnostic>>,

    /// The receiver side of the channel mentioned above.
    command_receiver: Option<Receiver<AptosDiagnostic>>,

    diagnostics_cleared_for_all: bool,
    diagnostics_received: bool,
}

#[allow(clippy::large_enum_variant)]
enum Event {
    RequestStateChange(StateChange),
    CheckEvent(Option<AptosDiagnostic>),
}

impl FlycheckActor {
    fn new(
        ws_id: usize,
        sender: Sender<FlycheckMessage>,
        config: FlycheckConfig,
        workspace_root: AbsPathBuf,
    ) -> FlycheckActor {
        tracing::info!(%ws_id, ?workspace_root, "Spawning flycheck");
        FlycheckActor {
            ws_id,
            sender,
            config,
            root: Arc::new(workspace_root),
            command_handle: None,
            command_receiver: None,
            diagnostics_cleared_for_all: false,
            diagnostics_received: false,
        }
    }

    fn report_progress(&self, progress: Progress) {
        self.send(FlycheckMessage::Progress { ws_id: self.ws_id, progress });
    }

    fn next_event(&self, inbox: &Receiver<StateChange>) -> Option<Event> {
        let Some(command_receiver) = &self.command_receiver else {
            return inbox.recv().ok().map(Event::RequestStateChange);
        };

        // Biased to give restarts a preference so check outputs don't block a restart or stop
        select_biased! {
            recv(inbox) -> msg => msg.ok().map(Event::RequestStateChange),
            recv(command_receiver) -> msg => Some(Event::CheckEvent(msg.ok())),
        }
    }

    fn run(mut self, inbox: Receiver<StateChange>) {
        'event: while let Some(event) = self.next_event(&inbox) {
            match event {
                Event::RequestStateChange(StateChange::Cancel) => {
                    tracing::info!(flycheck_id = self.ws_id, "flycheck cancelled");
                    self.cancel_check_process();
                }
                Event::RequestStateChange(StateChange::Restart) => {
                    // Cancel the previously spawned process
                    self.cancel_check_process();
                    while let Ok(restart) = inbox.recv_timeout(Duration::from_millis(50)) {
                        // restart chained with a stop, so just cancel
                        if let StateChange::Cancel = restart {
                            continue 'event;
                        }
                    }

                    let command = self.flycheck_command();

                    let formatted_command = format!("{command:?}");
                    tracing::info!("will restart flycheck");

                    let (sender, receiver) = unbounded();
                    match CommandHandle::spawn(command, sender) {
                        Ok(command_handle) => {
                            tracing::info!(command = %formatted_command, "did restart flycheck");
                            self.command_handle = Some(command_handle);
                            self.command_receiver = Some(receiver);
                            self.report_progress(Progress::DidStart);
                        }
                        Err(error) => {
                            self.report_progress(Progress::DidFailToRestart(format!(
                                "Failed to run the following command: {formatted_command} error={error}"
                            )));
                        }
                    }
                }
                Event::CheckEvent(None) => {
                    tracing::info!(flycheck_id = self.ws_id, "flycheck finished");

                    // Watcher finished
                    let command_handle = self.command_handle.take().unwrap();
                    self.command_receiver.take();
                    let formatted_handle = format!("{command_handle:?}");

                    let res = command_handle.join();
                    if let Err(error) = &res {
                        tracing::info!(
                            "Flycheck failed to run the following command: {}, error={}",
                            formatted_handle,
                            error
                        );
                    }
                    if !self.diagnostics_received {
                        tracing::debug!(flycheck_id = self.ws_id, "clearing diagnostics");
                        // We finished without receiving any diagnostics.
                        // Clear everything for good measure
                        self.send(FlycheckMessage::ClearDiagnostics { ws_id: self.ws_id });
                    }
                    self.clear_diagnostics_state();

                    self.report_progress(Progress::DidFinish(res));
                }
                Event::CheckEvent(Some(diagnostic)) => {
                    tracing::debug!(
                        flycheck_id = self.ws_id,
                        message = diagnostic.message,
                        "diagnostic received"
                    );
                    self.diagnostics_received = true;

                    if !self.diagnostics_cleared_for_all {
                        self.diagnostics_cleared_for_all = true;
                        self.send(FlycheckMessage::ClearDiagnostics { ws_id: self.ws_id });
                    }

                    self.send(FlycheckMessage::AddDiagnostic {
                        ws_id: self.ws_id,
                        diagnostic,
                    });
                }
            }
        }
        // If we rerun the thread, we need to discard the previous check results first
        self.cancel_check_process();
    }

    fn cancel_check_process(&mut self) {
        if let Some(command_handle) = self.command_handle.take() {
            tracing::info!(
                command = ?command_handle,
                "did cancel flycheck"
            );
            command_handle.cancel();
            self.command_receiver.take();
            self.report_progress(Progress::DidCancel);
        }
        self.clear_diagnostics_state();
    }

    fn clear_diagnostics_state(&mut self) {
        self.diagnostics_cleared_for_all = false;
        self.diagnostics_received = false;
    }

    /// Construct a `Command` object for checking the user's code. If the user
    /// has specified a custom command with placeholders that we cannot fill,
    /// return None.
    fn flycheck_command(&self) -> Command {
        let FlycheckConfig {
            enabled: _,
            aptos_cli,
            command,
            options,
        } = &self.config;

        let mut cmd = toolchain::command(aptos_cli, &*self.root);

        cmd.arg("move");
        cmd.arg(command);
        if command == "compile" {
            cmd.arg("--optimize=none");
        }
        cmd.arg("--skip-fetch-latest-git-deps");
        cmd.arg("--experiments");
        cmd.arg("compiler-message-format-json");

        options.apply_on_command(&mut cmd);

        cmd
    }

    #[track_caller]
    fn send(&self, check_task: FlycheckMessage) {
        self.sender.send(check_task).unwrap();
    }
}
