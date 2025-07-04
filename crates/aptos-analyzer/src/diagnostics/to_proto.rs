// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::flycheck::compiler_diagnostic::DiagnosticLabel;
use crate::global_state::GlobalStateSnapshot;
use crate::lsp::to_proto::url_from_abs_path;
use line_index::LineCol;
use paths::AbsPathBuf;

#[derive(Debug)]
pub(crate) struct MappedAptosDiagnostic {
    pub(crate) url: lsp_types::Url,
    pub(crate) diagnostic: lsp_types::Diagnostic,
}

/// Converts a Rust root diagnostic to LSP form
///
/// This flattens the Rust diagnostic by:
///
/// 1. Creating a LSP diagnostic with the root message and primary span.
/// 2. Adding any labelled secondary spans to `relatedInformation`
/// 3. Categorising child diagnostics as either `SuggestedFix`es,
///    `relatedInformation` or additional message lines.
///
/// If the diagnostic has no primary span this will return `None`
pub(crate) fn map_aptos_diagnostic_to_lsp(
    diag: &crate::flycheck::AptosDiagnostic,
    snap: &GlobalStateSnapshot,
) -> anyhow::Result<MappedAptosDiagnostic> {
    let label = diag.labels.iter().find(|l| l.is_primary());
    if label.is_none() {
        return Err(anyhow::Error::msg("No primary label"));
    }

    let label = label.unwrap();
    let severity = aptos_diagnostic_severity(diag.severity.as_str());

    let code = diag.code.clone();
    // todo: ignore errors
    // if let Some(code_val) = &code {
    //     if config.check_ignore.contains(code_val) {
    //         return Vec::new();
    //     }
    // }

    // todo: look at the rust-analyzer function to expand contents of diagnostics

    let primary_location = location(label, snap)?;
    let message = diag.message.clone();

    Ok(MappedAptosDiagnostic {
        url: primary_location.uri.clone(),
        diagnostic: lsp_types::Diagnostic {
            range: primary_location.range,
            severity: Some(severity),
            code: code.clone().map(lsp_types::NumberOrString::String),
            code_description: None,
            source: snap
                .config
                .flycheck_config()
                .map(|it| format!("aptos move {}", it.command())),
            message,
            related_information: None,
            tags: None,
            data: None,
        },
    })
}

pub(crate) fn aptos_diagnostic_severity(severity: &str) -> lsp_types::DiagnosticSeverity {
    match severity {
        "Bug" | "Error" => lsp_types::DiagnosticSeverity::ERROR,
        "Warning [lint]" | "Warning" => lsp_types::DiagnosticSeverity::WARNING,
        _ => lsp_types::DiagnosticSeverity::INFORMATION,
    }
}

/// Converts a Rust span to a LSP location
fn location(
    code_label: &DiagnosticLabel,
    snap: &GlobalStateSnapshot,
) -> anyhow::Result<lsp_types::Location> {
    let label_abs_path = AbsPathBuf::try_from(code_label.file_id.as_str()).unwrap();
    let url = url_from_abs_path(label_abs_path.as_path());
    let file_id = snap.url_to_file_id(&url)?;

    let start_pos = position(
        snap.analysis
            .file_offset_into_position(file_id, code_label.range.start)?,
    );
    let end_pos = position(
        snap.analysis
            .file_offset_into_position(file_id, code_label.range.end)?,
    );

    let range = { lsp_types::Range::new(start_pos, end_pos) };
    Ok(lsp_types::Location::new(url, range))
}

fn position(line_col: LineCol) -> lsp_types::Position {
    lsp_types::Position::new(line_col.line, line_col.col)
}
