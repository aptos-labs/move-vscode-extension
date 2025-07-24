// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::config::MovefmtConfig;
use crate::global_state::GlobalStateSnapshot;
use crate::lsp::{LspError, from_proto, to_proto};
use crate::toolchain;
use anyhow::Context;
use ide_db::line_endings::LineEndings;
use ide_db::text_edit::TextEdit;
use lsp_types::TextDocumentIdentifier;
use regex::Regex;
use std::io;
use std::io::{ErrorKind, Write};
use std::process::{Command, Stdio};
use std::sync::LazyLock;
use syntax::{TextRange, TextSize};

pub(crate) fn run_movefmt(
    snap: &GlobalStateSnapshot,
    text_document: TextDocumentIdentifier,
) -> anyhow::Result<Option<Vec<lsp_types::TextEdit>>> {
    let file_id = from_proto::file_id(snap, &text_document.uri)?;
    let file_text = snap.analysis.file_text(file_id)?;

    let line_index = snap.file_line_index(file_id)?;

    // try to chdir to the file so we can respect `movefmt.toml`
    let current_dir = match text_document.uri.to_file_path() {
        Ok(mut path) => {
            // pop off file name
            if path.pop() && path.is_dir() {
                path
            } else {
                std::env::current_dir()?
            }
        }
        Err(_) => {
            tracing::error!(
                text_document = ?text_document.uri,
                "Unable to get path, movefmt.toml might be ignored"
            );
            std::env::current_dir()?
        }
    };

    let movefmt_config = match snap.config.movefmt() {
        Some(cfg) => cfg,
        None => {
            snap.ask_client_for_movefmt_update("movefmt is not provided".to_string());
            return Ok(None);
            // return Err(LspError::new(
            //     ErrorCode::RequestFailed as i32,
            //     String::from("movefmt path is not provided"),
            // )
            // .into());
        }
    };

    let current_version = get_movefmt_version(&movefmt_config)?;
    if current_version < semver::Version::new(1, 2, 1) {
        snap.ask_client_for_movefmt_update(format!("current version {current_version} < 1.2.1"));
        return Ok(None);
    }

    let mut command = toolchain::command(&movefmt_config.path, current_dir);
    command.arg("--quiet");
    command.arg("--stdin");
    command.args(vec!["--emit", "stdout"]);

    command.args(movefmt_config.extra_args);

    let command_line = format!("{:?}", &command);
    let output = {
        let _p = tracing::info_span!("movefmt").entered();
        tracing::info!(?command);

        let mut movefmt = spawn_command(command)?;
        movefmt.stdin.as_mut().unwrap().write_all(file_text.as_bytes())?;

        movefmt.wait_with_output()?
    };

    let captured_stdout = String::from_utf8(output.stdout)?;
    let captured_stderr = String::from_utf8(output.stderr).unwrap_or_default();

    if !output.status.success() {
        let stdout = strip_ansi_escapes::strip_str(&captured_stdout);
        let stderr = strip_ansi_escapes::strip_str(&captured_stderr);

        return match output.status.code() {
            Some(1) if stdout.contains("a valid move code") => {
                snap.show_message(
                    lsp_types::MessageType::ERROR,
                    "movefmt error: invalid syntax".to_string(),
                );
                Ok(None)
            }
            _ => {
                // Something else happened - e.g. `movefmt` is missing or caught a signal
                let error_message = format!(
                    r#"movefmt exited with:
                           Status: {}
                           command: {command_line}
                           stdout: {stdout}
                           stderr: {stderr}"#,
                    output.status,
                );
                Err(LspError::new(-32900, error_message).into())
            }
        };
    }

    let (new_text, _) = LineEndings::normalize(captured_stdout);

    if *file_text == new_text {
        Ok(None)
    } else {
        Ok(Some(to_proto::text_edit_vec(
            &line_index,
            TextEdit::replace(TextRange::up_to(TextSize::of(&*file_text)), new_text),
        )))
    }
}

fn get_movefmt_version(movefmt_config: &MovefmtConfig) -> anyhow::Result<semver::Version> {
    let mut command = toolchain::command(&movefmt_config.path, std::env::current_dir()?);
    command.arg("--version");
    command.env("MOVEFMT_LOG", "error");
    let process = spawn_command(command)?;

    let version_stdout: String = process.wait_with_output()?.stdout.try_into()?;
    let version = parse_movefmt_version(version_stdout.as_str());
    version.ok_or_else(|| {
        anyhow::Error::new(io::Error::new(
            ErrorKind::InvalidData,
            format!("Invalid movefmt version output {version_stdout:?}"),
        ))
    })
}

fn spawn_command(mut command: Command) -> anyhow::Result<std::process::Child> {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context(format!("Failed to spawn {command:?}"))
}

fn parse_movefmt_version(version_stdout: &str) -> Option<semver::Version> {
    let captures = VERSION_REGEX.captures(version_stdout)?;
    let version = captures.get(1)?;
    semver::Version::parse(version.as_str()).ok()
}

static VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"movefmt v(.+)").unwrap());

#[cfg(test)]
mod tests {
    use super::*;
    use semver::Version;

    #[test]
    fn test_parse_movefmt_version() {
        let version = parse_movefmt_version("movefmt v1.2.1".into()).unwrap();
        assert_eq!(version, Version::new(1, 2, 1));
    }
}
