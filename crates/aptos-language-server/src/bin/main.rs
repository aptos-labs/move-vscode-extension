// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

//! Driver for aptos-language-server.

use std::{env, fs, path::PathBuf, process::ExitCode, sync::Arc};

use anyhow::Context;
use aptos_language_server::cli::{AptosAnalyzerCmd, CliArgs};
use aptos_language_server::{Config, ConfigChange, ConfigErrors, from_json};
use clap::Parser;
use lsp_server::Connection;
use paths::Utf8PathBuf;
use tracing::Level;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use vfs::AbsPathBuf;

fn main() -> anyhow::Result<ExitCode> {
    let args = CliArgs::parse();

    if let Err(e) = setup_logging(args.log_file.clone()) {
        eprintln!("Failed to setup logging: {e:#}");
    }

    match args.subcommand {
        None => {
            if args.version {
                println!("aptos-language-server {}", aptos_language_server::version());
            }
        }
        Some(AptosAnalyzerCmd::LspServer) => 'lsp_server: {
            if args.version {
                println!("aptos-language-server {}", aptos_language_server::version());
                break 'lsp_server;
            }

            // rust-analyzer’s “main thread” is actually
            // a secondary latency-sensitive thread with an increased stack size.
            // We use this thread intent because any delay in the main loop
            // will make actions like hitting enter in the editor slow.
            with_extra_thread(
                "LspServer",
                stdx::thread::ThreadIntent::LatencySensitive,
                run_server,
            )?;
        }
        Some(AptosAnalyzerCmd::Diagnostics(cmd)) => {
            let exit_code = cmd.run()?;
            return Ok(exit_code);
        }
        Some(AptosAnalyzerCmd::Bench(cmd)) => {
            let exit_code = cmd.run()?;
            return Ok(exit_code);
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn setup_logging(log_file_option: Option<PathBuf>) -> anyhow::Result<()> {
    if cfg!(windows) {
        // This is required so that windows finds our pdb that is placed right beside the exe.
        // By default it doesn't look at the folder the exe resides in, only in the current working
        // directory which we set to the project workspace.
        // https://docs.microsoft.com/en-us/windows-hardware/drivers/debugger/general-environment-variables
        // https://docs.microsoft.com/en-us/windows/win32/api/dbghelp/nf-dbghelp-syminitialize
        if let Ok(path) = env::current_exe() {
            if let Some(path) = path.parent() {
                unsafe {
                    env::set_var("_NT_SYMBOL_PATH", path);
                }
            }
        }
    }

    if env::var("RUST_BACKTRACE").is_err() {
        unsafe {
            env::set_var("RUST_BACKTRACE", "short");
        }
    }

    let log_file = env::var("RA_LOG_FILE")
        .ok()
        .map(PathBuf::from)
        .or(log_file_option);
    let log_file = match log_file {
        Some(path) => {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            Some(
                fs::File::create(&path)
                    .with_context(|| format!("can't create log file at {}", path.display()))?,
            )
        }
        None => None,
    };
    let writer = match log_file {
        Some(file) => BoxMakeWriter::new(Arc::new(file)),
        None => BoxMakeWriter::new(std::io::stderr),
    };

    aptos_language_server::tracing::LoggingConfig {
        writer,
        // Deliberately enable all `error` logs if the user has not set RA_LOG, as there is usually
        // useful information in there for debugging.
        default_level: Level::ERROR,
    }
    .try_init()?;

    Ok(())
}

const STACK_SIZE: usize = 1024 * 1024 * 8;

/// Parts of rust-analyzer can use a lot of stack space, and some operating systems only give us
/// 1 MB by default (eg. Windows), so this spawns a new thread with hopefully sufficient stack
/// space.
fn with_extra_thread(
    thread_name: impl Into<String>,
    thread_intent: stdx::thread::ThreadIntent,
    f: impl FnOnce() -> anyhow::Result<()> + Send + 'static,
) -> anyhow::Result<()> {
    let handle = stdx::thread::Builder::new(thread_intent, thread_name)
        .stack_size(STACK_SIZE)
        .spawn(f)?;

    handle.join()?;

    Ok(())
}

fn run_server() -> anyhow::Result<()> {
    tracing::info!("server version {} will start", aptos_language_server::version());

    let (connection, io_threads, mut config) = initialization_handshake()?;

    config.rediscover_packages();

    if !hide_init_params() {
        tracing::info!("initial config: {:#?}", config);
    }

    // blocks
    let main_loop_result = aptos_language_server::main_loop(config, connection);

    let io_threads_result = io_threads.join();
    // If the io_threads have an error, there's usually an error on the main
    // loop too because the channels are closed. Ensure we report both errors.
    match (main_loop_result, io_threads_result) {
        (Err(loop_e), Err(join_e)) => anyhow::bail!("{loop_e}\n{join_e}"),
        (Ok(_), Err(join_e)) => anyhow::bail!("{join_e}"),
        (Err(loop_e), Ok(_)) => anyhow::bail!("{loop_e}"),
        (Ok(_), Ok(_)) => {}
    }

    tracing::info!("server did shut down");
    Ok(())
}

fn initialization_handshake() -> anyhow::Result<(Connection, lsp_server::IoThreads, Config)> {
    let (connection, io_threads) = Connection::stdio();

    let (initialize_id, initialize_params) = match connection.initialize_start() {
        Ok(it) => it,
        Err(e) => {
            if e.channel_is_disconnected() {
                io_threads.join()?;
            }
            return Err(e.into());
        }
    };

    let hide_log_init_params = env::var("APT_LOG_HIDE_INIT_PARAMS").is_ok();
    if hide_log_init_params {
        tracing::info!(
            "LSP initialization params are hidden. To show them, unset \"APT_LOG_HIDE_INIT_PARAMS\" environment variable.",
        )
    }
    if !hide_log_init_params {
        tracing::info!("InitializeParams: {}", initialize_params);
    }

    let initialize_params =
        from_json::<lsp_types::InitializeParams>("InitializeParams", &initialize_params)?;
    #[allow(deprecated)]
    let lsp_types::InitializeParams {
        root_uri,
        capabilities,
        workspace_folders,
        initialization_options,
        client_info,
        ..
    } = initialize_params;

    if let Some(client_info) = &client_info {
        tracing::info!(
            "Client '{}' {}",
            client_info.name,
            client_info.version.as_deref().unwrap_or_default()
        );
    }

    let root_path = match root_uri
        .and_then(|it| it.to_file_path().ok())
        .map(patch_path_prefix)
        .and_then(|it| Utf8PathBuf::from_path_buf(it).ok())
        .and_then(|it| AbsPathBuf::try_from(it).ok())
    {
        Some(it) => it,
        None => {
            let cwd = env::current_dir()?;
            AbsPathBuf::assert_utf8(cwd)
        }
    };

    let workspace_roots = workspace_folders
        .map(|workspace_folders| {
            workspace_folders
                .into_iter()
                .filter_map(|it| it.uri.to_file_path().ok())
                .map(patch_path_prefix)
                .filter_map(|it| Utf8PathBuf::from_path_buf(it).ok())
                .filter_map(|it| AbsPathBuf::try_from(it).ok())
                .collect::<Vec<_>>()
        })
        .filter(|roots| !roots.is_empty())
        .unwrap_or_else(|| vec![root_path.clone()]);
    tracing::info!(?workspace_roots);

    let mut config = Config::new(root_path, capabilities, workspace_roots, client_info);
    if let Some(json) = initialization_options {
        let mut change = ConfigChange::default();
        change.change_client_config(json);

        let error_sink: ConfigErrors;
        (config, error_sink) = config.apply_change(change);

        if !error_sink.is_empty() {
            use lsp_types::{
                MessageType, ShowMessageParams,
                notification::{Notification, ShowMessage},
            };
            let not = lsp_server::Notification::new(
                ShowMessage::METHOD.to_owned(),
                ShowMessageParams {
                    typ: MessageType::WARNING,
                    message: error_sink.to_string(),
                },
            );
            connection
                .sender
                .send(lsp_server::Message::Notification(not))
                .unwrap();
        }
    }

    let server_capabilities = aptos_language_server::server_capabilities(&config);

    let initialize_result = lsp_types::InitializeResult {
        capabilities: server_capabilities,
        server_info: Some(lsp_types::ServerInfo {
            name: String::from("aptos-language-server"),
            version: Some(aptos_language_server::version().to_string()),
        }),
        offset_encoding: None,
    };

    let initialize_result = serde_json::to_value(initialize_result).unwrap();

    if let Err(e) = connection.initialize_finish(initialize_id, initialize_result) {
        if e.channel_is_disconnected() {
            io_threads.join()?;
        }
        return Err(e.into());
    }

    Ok((connection, io_threads, config))
}

fn hide_init_params() -> bool {
    env::var("APT_LOG_HIDE_INIT_PARAMS").is_ok()
}

fn patch_path_prefix(path: PathBuf) -> PathBuf {
    use std::path::{Component, Prefix};
    if cfg!(windows) {
        // VSCode might report paths with the file drive in lowercase, but this can mess
        // with env vars set by tools and build scripts executed by r-a such that it invalidates
        // cargo's compilations unnecessarily. https://github.com/rust-lang/rust-analyzer/issues/14683
        // So we just uppercase the drive letter here unconditionally.
        // (doing it conditionally is a pain because std::path::Prefix always reports uppercase letters on windows)
        let mut comps = path.components();
        match comps.next() {
            Some(Component::Prefix(prefix)) => {
                let prefix = match prefix.kind() {
                    Prefix::Disk(d) => {
                        format!("{}:", d.to_ascii_uppercase() as char)
                    }
                    Prefix::VerbatimDisk(d) => {
                        format!(r"\\?\{}:", d.to_ascii_uppercase() as char)
                    }
                    _ => return path,
                };
                let mut path = PathBuf::new();
                path.push(prefix);
                path.extend(comps);
                path
            }
            _ => path,
        }
    } else {
        path
    }
}

#[test]
#[cfg(windows)]
fn patch_path_prefix_works() {
    assert_eq!(
        patch_path_prefix(r"c:\foo\bar".into()),
        PathBuf::from(r"C:\foo\bar")
    );
    assert_eq!(
        patch_path_prefix(r"\\?\c:\foo\bar".into()),
        PathBuf::from(r"\\?\C:\foo\bar")
    );
}
