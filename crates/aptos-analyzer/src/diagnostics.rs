pub(crate) mod to_proto;

use crate::global_state::GlobalStateSnapshot;
use crate::lsp;
use crate::main_loop::DiagnosticsTaskKind;
use base_db::source_db::DbPanicContext;
use std::collections::{HashMap, HashSet};
use std::mem;
use stdx::iter_eq_by;
use stdx::itertools::Itertools;
use vfs::FileId;

pub(crate) type DiagnosticsGeneration = usize;

#[derive(Debug, Default, Clone)]
pub(crate) struct DiagnosticCollection {
    // FIXME: should be IntMap<FileId, Vec<ra_id::Diagnostic>>
    pub(crate) native_syntax: HashMap<FileId, (DiagnosticsGeneration, Vec<lsp_types::Diagnostic>)>,
    pub(crate) native_semantic: HashMap<FileId, (DiagnosticsGeneration, Vec<lsp_types::Diagnostic>)>,

    // flycheck_id (ws_id) -> (file_id -> Vec<Diagnostic>)
    pub(crate) flycheck: HashMap<usize, HashMap<FileId, Vec<lsp_types::Diagnostic>>>,
    changes: HashSet<FileId>,

    /// Counter for supplying a new generation number for diagnostics.
    /// This is used to keep track of when to clear the diagnostics for a given file as we compute
    /// diagnostics on multiple worker threads simultaneously which may result in multiple diagnostics
    /// updates for the same file in a single generation update (due to macros affecting multiple files).
    generation: DiagnosticsGeneration,
}

impl DiagnosticCollection {
    pub(crate) fn clear_flycheck(&mut self, flycheck_id: usize) {
        if let Some(check) = self.flycheck.get_mut(&flycheck_id) {
            let drained_keys = check.drain().map(|(k, _)| k.to_owned());
            self.changes.extend(drained_keys)
        }
    }

    pub(crate) fn clear_flycheck_all(&mut self) {
        for files_diags in self.flycheck.values_mut() {
            let drained_keys = files_diags.drain().map(|(k, _)| k.to_owned());
            self.changes.extend(drained_keys);
        }
    }

    pub(crate) fn clear_native_for(&mut self, file_id: FileId) {
        self.native_syntax.remove(&file_id);
        self.native_semantic.remove(&file_id);
        self.changes.insert(file_id);
    }

    pub(crate) fn add_flycheck_diagnostic(
        &mut self,
        flycheck_id: usize,
        file_id: FileId,
        diagnostic: lsp_types::Diagnostic,
    ) {
        let existing_diagnostics = self
            .flycheck
            .entry(flycheck_id)
            .or_default()
            .entry(file_id)
            .or_default();
        for existing_diagnostic in existing_diagnostics.iter() {
            if are_diagnostics_equal(existing_diagnostic, &diagnostic) {
                return;
            }
        }

        existing_diagnostics.push(diagnostic);
        self.changes.insert(file_id);
    }

    pub(crate) fn set_native_diagnostics(&mut self, kind: DiagnosticsTaskKind) {
        let (generation, diagnostics, target) = match kind {
            DiagnosticsTaskKind::Syntax(generation, diagnostics) => {
                (generation, diagnostics, &mut self.native_syntax)
            }
            DiagnosticsTaskKind::Semantic(generation, diagnostics) => {
                (generation, diagnostics, &mut self.native_semantic)
            }
        };

        for (file_id, mut diagnostics) in diagnostics {
            diagnostics.sort_by_key(|it| (it.range.start, it.range.end));

            if let Some((old_gen, existing_diagnostics)) = target.get_mut(&file_id) {
                if existing_diagnostics.len() == diagnostics.len()
                    && iter_eq_by(&diagnostics, &*existing_diagnostics, |new, existing| {
                        are_diagnostics_equal(new, existing)
                    })
                {
                    // don't signal an update if the diagnostics are the same
                    continue;
                }
                if *old_gen < generation || generation == 0 {
                    target.insert(file_id, (generation, diagnostics));
                } else {
                    existing_diagnostics.extend(diagnostics);
                    existing_diagnostics.sort_by_key(|it| (it.range.start, it.range.end))
                }
            } else {
                target.insert(file_id, (generation, diagnostics));
            }
            self.changes.insert(file_id);
        }
    }

    pub(crate) fn diagnostics_for(
        &self,
        file_id: FileId,
    ) -> impl Iterator<Item = &lsp_types::Diagnostic> {
        let native_syntax = self.native_syntax.get(&file_id).into_iter().flat_map(|(_, d)| d);
        let native_semantic = self
            .native_semantic
            .get(&file_id)
            .into_iter()
            .flat_map(|(_, d)| d);
        let check = self
            .flycheck
            .values()
            .filter_map(move |it| it.get(&file_id))
            .flatten();
        native_syntax.chain(native_semantic).chain(check)
    }

    pub(crate) fn take_changes(&mut self) -> Option<HashSet<FileId>> {
        if self.changes.is_empty() {
            return None;
        }
        Some(mem::take(&mut self.changes))
    }

    pub(crate) fn next_generation(&mut self) -> usize {
        self.generation += 1;
        self.generation
    }
}

fn are_diagnostics_equal(left: &lsp_types::Diagnostic, right: &lsp_types::Diagnostic) -> bool {
    left.source == right.source
        && left.severity == right.severity
        && left.range == right.range
        && left.message == right.message
}

pub(crate) enum NativeDiagnosticsFetchKind {
    Syntax,
    Semantic,
}

pub(crate) fn fetch_native_diagnostics(
    snapshot: &GlobalStateSnapshot,
    subscriptions: std::sync::Arc<[FileId]>,
    slice: std::ops::Range<usize>,
    kind: NativeDiagnosticsFetchKind,
) -> Vec<(FileId, Vec<lsp_types::Diagnostic>)> {
    let _p = tracing::info_span!("fetch_native_diagnostics").entered();
    let _ctx = DbPanicContext::enter("fetch_native_diagnostics".to_owned());

    // the diagnostics produced may point to different files not requested by the concrete request,
    // put those into here and filter later
    let mut odd_ones = Vec::new();
    let mut diagnostics = subscriptions[slice]
        .iter()
        .copied()
        .filter_map(|file_id| {
            let line_index = snapshot.file_line_index(file_id).ok()?;
            let config = &snapshot.config.diagnostics_config();
            let frange = snapshot.full_range(file_id).ok()?;
            let diagnostics = match kind {
                NativeDiagnosticsFetchKind::Syntax => {
                    snapshot.analysis.syntax_diagnostics(config, file_id).ok()?
                }
                NativeDiagnosticsFetchKind::Semantic if config.enabled => snapshot
                    .analysis
                    .semantic_diagnostics(config, ide::AssistResolveStrategy::None, frange)
                    .ok()?,
                NativeDiagnosticsFetchKind::Semantic => return None,
            };
            let diagnostics = diagnostics
                .into_iter()
                .filter_map(|d| {
                    if d.range.file_id == file_id {
                        Some(to_proto_diagnostic(&line_index, d))
                    } else {
                        odd_ones.push(d);
                        None
                    }
                })
                .collect::<Vec<_>>();
            Some((file_id, diagnostics))
        })
        .collect::<Vec<_>>();

    // Add back any diagnostics that point to files we are subscribed to
    for (file_id, group) in odd_ones
        .into_iter()
        .sorted_by_key(|it| it.range.file_id)
        .chunk_by(|it| it.range.file_id)
        .into_iter()
    {
        if !subscriptions.contains(&file_id) {
            continue;
        }
        let Some((_, diagnostics)) = diagnostics.iter_mut().find(|&&mut (id, _)| id == file_id) else {
            continue;
        };
        let Some(line_index) = snapshot.file_line_index(file_id).ok() else {
            break;
        };
        for diagnostic in group {
            diagnostics.push(to_proto_diagnostic(&line_index, diagnostic));
        }
    }
    diagnostics
}

pub(crate) fn to_proto_diagnostic(
    line_index: &syntax::line_index::LineIndex,
    d: ide_diagnostics::diagnostic::Diagnostic,
) -> lsp_types::Diagnostic {
    lsp_types::Diagnostic {
        range: lsp::to_proto::lsp_range(line_index, d.range.range),
        severity: Some(lsp::to_proto::diagnostic_severity(d.severity)),
        code: Some(lsp_types::NumberOrString::String(d.code.as_str().to_owned())),
        code_description: None,
        // code_description: Some(lsp_types::CodeDescription {
        //     href: lsp_types::Url::parse(&d.code.url()).unwrap(),
        // }),
        source: Some("aptos-analyzer".to_owned()),
        message: d.message,
        related_information: None,
        tags: d.unused.then(|| vec![lsp_types::DiagnosticTag::UNNECESSARY]),
        data: None,
    }
}
