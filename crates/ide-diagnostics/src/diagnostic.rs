// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use ide_db::Severity;
use ide_db::assist_context::LocalAssists;
use ide_db::assists::Assist;
use syntax::files::FileRange;
use vfs::FileId;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub range: FileRange,
    pub severity: Severity,
    pub unused: bool,
    pub fixes: Option<Vec<Assist>>,
}

impl Diagnostic {
    pub fn new(
        code: DiagnosticCode,
        message: impl Into<String>,
        range: impl Into<FileRange>,
    ) -> Diagnostic {
        let message = message.into();
        Diagnostic {
            code,
            message,
            range: range.into(),
            severity: match code {
                DiagnosticCode::SyntaxError => Severity::Error,
                DiagnosticCode::Lsp(_, s) => s,
            },
            unused: false,
            fixes: None,
        }
    }

    pub fn new_syntax_error(file_id: FileId, err: &syntax::SyntaxError) -> Diagnostic {
        Diagnostic::new(
            DiagnosticCode::SyntaxError,
            format!("Syntax Error: {err}"),
            FileRange {
                file_id: file_id.into(),
                range: err.range(),
            },
        )
    }

    pub(crate) fn with_local_fixes(mut self, fixes: Option<LocalAssists>) -> Diagnostic {
        self.fixes = fixes.map(|it| it.assists());
        self
    }

    pub(crate) fn with_unused(mut self, unused: bool) -> Diagnostic {
        self.unused = unused;
        self
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    SyntaxError,
    Lsp(&'static str, Severity),
}

impl DiagnosticCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagnosticCode::Lsp(r, _) => r,
            DiagnosticCode::SyntaxError => "syntax-error",
        }
    }
}
