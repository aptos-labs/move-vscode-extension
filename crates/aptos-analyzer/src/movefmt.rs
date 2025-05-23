use crate::global_state::GlobalStateSnapshot;
use crate::line_index::LineEndings;
use crate::lsp::{LspError, from_proto, to_proto};
use crate::toolchain;
use anyhow::Context;
use ide_db::text_edit::TextEdit;
use lsp_server::ErrorCode;
use lsp_types::TextDocumentIdentifier;
use std::io::Write;
use std::process::Stdio;
use syntax::{TextRange, TextSize};

pub(crate) fn run_movefmt(
    snap: &GlobalStateSnapshot,
    text_document: TextDocumentIdentifier,
) -> anyhow::Result<Option<Vec<lsp_types::TextEdit>>> {
    let file_id = from_proto::file_id(snap, &text_document.uri)?;
    let file_text = snap.analysis.file_text(file_id)?;

    let line_index = snap.file_line_index(file_id)?;

    // try to chdir to the file so we can respect `movefmt.toml`
    let mut file_path = None;
    let current_dir = match text_document.uri.to_file_path() {
        Ok(mut path) => {
            file_path = Some(path.clone());
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
            return Err(LspError::new(
                ErrorCode::RequestFailed as i32,
                String::from("movefmt path is not provided"),
            )
            .into());
        }
    };

    let mut command = toolchain::command(&movefmt_config.path, current_dir);
    command.arg("--quiet");
    command.arg("--stdin");
    command.args(vec!["--emit", "stdout"]);

    command.args(movefmt_config.extra_args);

    let command_line = format!("{:?}", &command);
    let output = {
        let _p = tracing::info_span!("movefmt").entered();
        tracing::info!(?command);

        let mut movefmt = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context(format!("Failed to spawn {command:?}"))?;

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
                snap.show_message_to_client(
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
