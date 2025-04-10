use ide_db::Severity;
use ide_db::assists::Assist;
use syntax::files::FileRange;
use vfs::FileId;

#[derive(Debug)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub range: FileRange,
    pub severity: Severity,
    pub unused: bool,
    // pub experimental: bool,
    pub fixes: Option<Vec<Assist>>,
    // The node that will be affected by `#[allow]` and similar attributes.
    // pub main_node: Option<InFile<SyntaxNodePtr>>,
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
                /*DiagnosticCode::RustcHardError(_) |*/
                DiagnosticCode::SyntaxError => Severity::Error,
                // FIXME: Rustc lints are not always warning, but the ones that are currently implemented are all warnings.
                // DiagnosticCode::RustcLint(_) => Severity::Warning,
                // FIXME: We can make this configurable, and if the user uses `cargo clippy` on flycheck, we can
                // make it normal warning.
                // DiagnosticCode::Clippy(_) => Severity::WeakWarning,
                DiagnosticCode::Lsp(_, s) => s,
            },
            unused: false,
            // experimental: false,
            fixes: None,
            // main_node: None,
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

    // fn new_with_syntax_node_ptr(
    //     ctx: &DiagnosticsContext<'_>,
    //     code: DiagnosticCode,
    //     message: impl Into<String>,
    //     node: InFile<SyntaxNodePtr>,
    // ) -> Diagnostic {
    //     Diagnostic::new(code, message, ctx.sema.diagnostics_display_range(node))
    //         .with_main_node(node)
    // }

    // fn experimental(mut self) -> Diagnostic {
    //     self.experimental = true;
    //     self
    // }

    // fn with_main_node(mut self, main_node: InFile<SyntaxNodePtr>) -> Diagnostic {
    //     self.main_node = Some(main_node);
    //     self
    // }

    pub(crate) fn with_fixes(mut self, fixes: Option<Vec<Assist>>) -> Diagnostic {
        self.fixes = fixes;
        self
    }

    fn with_unused(mut self, unused: bool) -> Diagnostic {
        self.unused = unused;
        self
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    SyntaxError,
    // Clippy(&'static str),
    Lsp(&'static str, Severity),
}

impl DiagnosticCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            // | DiagnosticCode::Clippy(r)
            DiagnosticCode::Lsp(r, _) => r,
            DiagnosticCode::SyntaxError => "syntax-error",
        }
    }
}
