#![allow(dead_code)]

use base_db::SourceDatabase;
use ide_db::{RootDatabase, Severity};
use syntax::files::FileRange;
use vfs::FileId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    // RustcHardError(&'static str),
    SyntaxError,
    // RustcLint(&'static str),
    // Clippy(&'static str),
    // Ra(&'static str, Severity),
}

impl DiagnosticCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            // DiagnosticCode::RustcHardError(r)
            // | DiagnosticCode::RustcLint(r)
            // | DiagnosticCode::Clippy(r)
            // | DiagnosticCode::Ra(r, _) => r,
            DiagnosticCode::SyntaxError => "syntax-error",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub range: FileRange,
    pub severity: Severity,
    pub unused: bool,
    // pub experimental: bool,
    // pub fixes: Option<Vec<Assist>>,
    // The node that will be affected by `#[allow]` and similar attributes.
    // pub main_node: Option<InFile<SyntaxNodePtr>>,
}

impl Diagnostic {
    fn new(code: DiagnosticCode, message: impl Into<String>, range: impl Into<FileRange>) -> Diagnostic {
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
                // DiagnosticCode::Ra(_, s) => s,
            },
            unused: false,
            // experimental: false,
            // fixes: None,
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

    // fn with_fixes(mut self, fixes: Option<Vec<Assist>>) -> Diagnostic {
    //     self.fixes = fixes;
    //     self
    // }

    fn with_unused(mut self, unused: bool) -> Diagnostic {
        self.unused = unused;
        self
    }
}

#[derive(Debug, Clone)]
pub struct DiagnosticsConfig {
    /// Whether native diagnostics are enabled.
    pub enabled: bool,
    // pub proc_macros_enabled: bool,
    // pub proc_attr_macros_enabled: bool,
    // pub disable_experimental: bool,
    // pub disabled: FxHashSet<String>,
    // pub expr_fill_default: ExprFillDefaultMode,
    // pub style_lints: bool,
    // FIXME: We may want to include a whole `AssistConfig` here
    // pub snippet_cap: Option<SnippetCap>,
    // pub insert_use: InsertUseConfig,
    // pub prefer_no_std: bool,
    // pub prefer_prelude: bool,
    // pub prefer_absolute: bool,
    // pub term_search_fuel: u64,
    // pub term_search_borrowck: bool,
}

impl DiagnosticsConfig {
    pub fn test_sample() -> Self {
        Self {
            enabled: true,
            // disable_experimental: Default::default(),
            // disabled: Default::default(),
            // expr_fill_default: Default::default(),
            // style_lints: true,
            // snippet_cap: SnippetCap::new(true),
            // insert_use: InsertUseConfig {
            //     granularity: ImportGranularity::Preserve,
            //     enforce_granularity: false,
            //     prefix_kind: PrefixKind::Plain,
            //     group: false,
            //     skip_glob_imports: false,
            // },
            // prefer_no_std: false,
            // prefer_prelude: true,
            // prefer_absolute: false,
            // term_search_fuel: 400,
            // term_search_borrowck: true,
        }
    }
}

/// Request parser level diagnostics for the given [`FileId`].
pub fn syntax_diagnostics(
    db: &RootDatabase,
    _config: &DiagnosticsConfig,
    file_id: FileId,
) -> Vec<Diagnostic> {
    let _p = tracing::info_span!("syntax_diagnostics").entered();

    // [#3434] Only take first 128 errors to prevent slowing down editor/ide, the number 128 is chosen arbitrarily.
    db.parse_errors(file_id)
        .as_deref()
        .into_iter()
        .flatten()
        .take(128)
        .map(|err| {
            Diagnostic::new(
                DiagnosticCode::SyntaxError,
                format!("Syntax Error: {err}"),
                FileRange {
                    file_id: file_id.into(),
                    range: err.range(),
                },
            )
        })
        .collect()
}
