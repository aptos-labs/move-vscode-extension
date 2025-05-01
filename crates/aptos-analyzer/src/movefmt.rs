use crate::global_state::GlobalStateSnapshot;
use crate::line_index::LineEndings;
use crate::lsp::{LspError, from_proto, to_proto};
use crate::{toolchain, unwrap_or_return_default};
use anyhow::Context;
use camino::Utf8PathBuf;
use ide_db::text_edit::TextEdit;
use lsp_server::ErrorCode;
use lsp_types::TextDocumentIdentifier;
use std::io::Write;
use std::process::{Command, Stdio};
use syntax::{TextRange, TextSize};

pub(crate) fn run_movefmt(
    snap: &GlobalStateSnapshot,
    text_document: TextDocumentIdentifier,
) -> anyhow::Result<Option<Vec<lsp_types::TextEdit>>> {
    let file_id = from_proto::file_id(snap, &text_document.uri)?;
    let file_text = snap.analysis.file_text(file_id)?;

    let line_index = snap.file_line_index(file_id)?;

    // try to chdir to the file so we can respect `movefmt.toml`
    // FIXME: use `rustfmt --config-path` once
    // https://github.com/rust-lang/rustfmt/issues/4660 gets fixed
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
    command.env("MOVEFMT_LOG", "error");
    command.arg("--quiet");
    command.args(vec!["--emit", "stdout"]);
    // if let Some(file_path) = file_path {
    //     command.args(vec!["--file-path", file_path.to_str().unwrap()]);
    // }
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

        // movefmt.stdin.as_mut().unwrap().write_all(file.as_bytes())?;

        movefmt.wait_with_output()?
    };

    let captured_stdout = String::from_utf8(output.stdout)?;
    let captured_stderr = String::from_utf8(output.stderr).unwrap_or_default();

    if !output.status.success() {
        // let movefmt_not_installed =
        //     captured_stderr.contains("not installed") || captured_stderr.contains("not available");

        return match output.status.code() {
            // Some(1) /*if !movefmt_not_installed*/ => {
            //     // While `rustfmt` doesn't have a specific exit code for parse errors this is the
            //     // likely cause exiting with 1. Most Language Servers swallow parse errors on
            //     // formatting because otherwise an error is surfaced to the user on top of the
            //     // syntax error diagnostics they're already receiving. This is especially jarring
            //     // if they have format on save enabled.
            //     tracing::warn!(
            //         ?command,
            //         %captured_stderr,
            //         "movefmt exited with status 1"
            //     );
            //     Ok(None)
            // }
            // rustfmt panicked at lexing/parsing the file
            // Some(101)
            //     if !movefmt_not_installed
            //         && (captured_stderr.starts_with("error[")
            //             || captured_stderr.starts_with("error:")) =>
            // {
            //     Ok(None)
            // }
            _ => {
                // Something else happened - e.g. `movefmt` is missing or caught a signal
                Err(LspError::new(
                    -32900,
                    format!(
                        r#"movefmt exited with:
                           Status: {}
                           command: {command_line}
                           stdout: {captured_stdout}
                           stderr: {captured_stderr}"#,
                        output.status,
                    ),
                )
                .into())
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
    //
    // if line_index.endings != new_line_endings {
    //     // If line endings are different, send the entire file.
    //     // Diffing would not work here, as the line endings might be the only
    //     // difference.
    //     Ok(Some(to_proto::text_edit_vec(
    //         &line_index,
    //         TextEdit::replace(TextRange::up_to(TextSize::of(&*file)), new_text),
    //     )))
    // } else if *file == new_text {
    //     // The document is already formatted correctly -- no edits needed.
    //     Ok(None)
    // } else {
    //     Ok(Some(to_proto::text_edit_vec(&line_index, diff(&file, &new_text))))
    // }
}
