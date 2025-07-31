// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

pub(crate) mod to_proto;

use crate::lsp;

pub(crate) fn to_proto_diagnostic(
    line_index: &crate::line_index::LineIndex,
    d: ide_diagnostics::diagnostic::Diagnostic,
) -> lsp_types::Diagnostic {
    lsp_types::Diagnostic {
        range: lsp::to_proto::lsp_range(line_index, d.range.range),
        severity: Some(lsp::to_proto::diagnostic_severity(d.severity)),
        code: Some(lsp_types::NumberOrString::String(d.code.as_str().to_owned())),
        code_description: None,
        source: Some("aptos-language-server".to_owned()),
        message: d.message,
        related_information: None,
        tags: d.unused.then(|| vec![lsp_types::DiagnosticTag::UNNECESSARY]),
        data: None,
    }
}
