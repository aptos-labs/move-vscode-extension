#![allow(dead_code)]

use base_db::change::FileChanges;
use base_db::{SourceDatabase, source_db};
use ide_completion::item::CompletionItem;
use ide_db::{RootDatabase, root_db};
use line_index::{LineCol, LineIndex};
use std::sync::Arc;
use syntax::{SourceFile, TextRange, TextSize};
use vfs::FileId;

pub mod extend_selection;
mod goto_definition;
mod hover;
pub mod inlay_hints;
mod navigation_target;
mod references;
mod rename;
pub mod syntax_highlighting;
mod type_info;
mod view_syntax_tree;

use crate::hover::HoverResult;
use crate::inlay_hints::{InlayHint, InlayHintsConfig};
pub use crate::navigation_target::NavigationTarget;
use crate::references::ReferenceSearchResult;
pub use crate::syntax_highlighting::HlRange;
use base_db::inputs::{InternFileId, PackageMetadata};
use base_db::package_root::PackageId;
use ide_completion::config::CompletionConfig;
use ide_db::assist_config::AssistConfig;
pub use ide_db::assists::{Assist, AssistKind, AssistResolveStrategy};
use ide_db::rename::RenameError;
use ide_db::search::SearchScope;
use ide_db::source_change::SourceChange;
use ide_diagnostics::config::DiagnosticsConfig;
use ide_diagnostics::diagnostic::Diagnostic;
pub use salsa::Cancelled;
use syntax::files::{FilePosition, FileRange};

pub type Cancellable<T> = Result<T, Cancelled>;

/// Info associated with a text range.
#[derive(Debug)]
pub struct RangeInfo<T> {
    pub range: TextRange,
    pub info: T,
}

impl<T> RangeInfo<T> {
    pub fn new(range: TextRange, info: T) -> RangeInfo<T> {
        RangeInfo { range, info }
    }
}

/// `AnalysisHost` stores the current state of the world.
#[derive(Debug)]
pub struct AnalysisHost {
    db: RootDatabase,
}

impl AnalysisHost {
    pub fn new() -> AnalysisHost {
        AnalysisHost { db: RootDatabase::new() }
    }

    pub fn with_database(db: RootDatabase) -> AnalysisHost {
        AnalysisHost { db }
    }

    /// Returns a snapshot of the current state, which you can query for
    /// semantic information.
    pub fn analysis(&self) -> Analysis {
        Analysis { db: self.db.snapshot() }
    }

    /// Applies changes to the current state of the world. If there are
    /// outstanding snapshots, they will be canceled.
    pub fn apply_change(&mut self, change: FileChanges) {
        self.db.apply_change(change);
    }

    pub fn request_cancellation(&mut self) {
        self.db.request_cancellation();
    }
    pub fn raw_database(&self) -> &RootDatabase {
        &self.db
    }
    pub fn raw_database_mut(&mut self) -> &mut RootDatabase {
        &mut self.db
    }
}

impl Default for AnalysisHost {
    fn default() -> AnalysisHost {
        AnalysisHost::new(/*None*/)
    }
}

/// Analysis is a snapshot of a world state at a moment in time. It is the main
/// entry point for asking semantic information about the world. When the world
/// state is advanced using `AnalysisHost::apply_change` method, all existing
/// `Analysis` are canceled (most method return `Err(Canceled)`).
#[derive(Debug)]
pub struct Analysis {
    db: RootDatabase,
}

// As a general design guideline, `Analysis` API are intended to be independent
// from the language server protocol. That is, when exposing some functionality
// we should think in terms of "what API makes most sense" and not in terms of
// "what types LSP uses". Although currently LSP is the only consumer of the
// API, the API should in theory be usable as a library, or via a different
// protocol.
impl Analysis {
    pub fn new(db_snapshot: RootDatabase) -> Self {
        Analysis { db: db_snapshot }
    }

    pub fn package_id(&self, file_id: FileId) -> Cancellable<PackageId> {
        self.with_db(|db| db.file_package_id(file_id))
    }

    pub fn package_metadata(&self, file_id: FileId) -> Cancellable<Option<PackageMetadata>> {
        self.with_db(|db| {
            let package_id = db.file_package_id(file_id);
            db.package_root(package_id)
                .data(db)
                .manifest_file_id
                .map(|file_id| db.package_metadata(file_id).metadata(db))
        })
    }

    pub fn is_local_package(&self, file_id: FileId) -> Cancellable<bool> {
        self.with_db(|db| {
            let package_id = db.file_package_id(file_id);
            let root = db.package_root(package_id).data(db);
            !root.is_library()
        })
    }

    /// Gets the text of the source file.
    pub fn file_text(&self, file_id: FileId) -> Cancellable<Arc<str>> {
        self.with_db(|db| SourceDatabase::file_text(db, file_id).text(db))
    }

    /// Gets the text of the source file.
    pub fn full_file_range(&self, file_id: FileId) -> Cancellable<FileRange> {
        let file_text = self.file_text(file_id)?;
        let frange = FileRange {
            file_id,
            range: TextRange::up_to(TextSize::of(&*file_text)),
        };
        Ok(frange)
    }

    /// Gets the syntax tree of the file.
    pub fn parse(&self, file_id: FileId) -> Cancellable<SourceFile> {
        self.with_db(|db| source_db::parse(db, file_id.intern(db)).tree())
    }

    /// Gets the file's `LineIndex`: data structure to convert between absolute
    /// offsets and line/column representation.
    pub fn file_line_index(&self, file_id: FileId) -> Cancellable<Arc<LineIndex>> {
        self.with_db(|db| root_db::line_index(db, file_id))
    }

    /// Selects the next syntactic nodes encompassing the range.
    pub fn extend_selection(&self, frange: FileRange) -> Cancellable<TextRange> {
        self.with_db(|db| extend_selection::extend_selection(db, frange))
    }

    // /// Returns position of the matching brace (all types of braces are
    // /// supported).
    // pub fn matching_brace(&self, position: FilePosition) -> Cancellable<Option<TextSize>> {
    //     self.with_db(|db| {
    //         let parse = db.parse(EditionedFileId::current_edition(position.file_id));
    //         let file = parse.tree();
    //         matching_brace::matching_brace(&file, position.offset)
    //     })
    // }

    pub fn view_syntax_tree(&self, file_id: FileId) -> Cancellable<String> {
        self.with_db(|db| view_syntax_tree::view_syntax_tree(db, file_id))
    }

    // pub const SUPPORTED_TRIGGER_CHARS: &'static str = typing::TRIGGER_CHARS;

    // /// Returns an edit which should be applied after a character was typed.
    // ///
    // /// This is useful for some on-the-fly fixups, like adding `;` to `let =`
    // /// automatically.
    // pub fn on_char_typed(
    //     &self,
    //     position: FilePosition,
    //     char_typed: char,
    // ) -> Cancellable<Option<SourceChange>> {
    //     // Fast path to not even parse the file.
    //     if !typing::TRIGGER_CHARS.contains(char_typed) {
    //         return Ok(None);
    //     }
    //
    //     self.with_db(|db| typing::on_char_typed(db, position, char_typed))
    // }

    // /// Returns a tree representation of symbols in the file. Useful to draw a
    // /// file outline.
    // pub fn file_structure(&self, file_id: FileId) -> Cancellable<Vec<StructureNode>> {
    //     self.with_db(|db| {
    //         file_structure::file_structure(
    //             &db.parse(EditionedFileId::current_edition(file_id)).tree(),
    //         )
    //     })
    // }

    /// Returns a list of the places in the file where type hints can be displayed.
    pub fn inlay_hints(
        &self,
        config: &InlayHintsConfig,
        file_id: FileId,
        range: Option<TextRange>,
    ) -> Cancellable<Vec<InlayHint>> {
        self.with_db(|db| inlay_hints::inlay_hints(db, file_id, range, config))
    }

    // pub fn inlay_hints_resolve(
    //     &self,
    //     config: &InlayHintsConfig,
    //     file_id: FileId,
    //     resolve_range: TextRange,
    //     hash: u64,
    //     hasher: impl Fn(&InlayHint) -> u64 + Send + UnwindSafe,
    // ) -> Cancellable<Option<InlayHint>> {
    //     self.with_db(|db| {
    //         inlay_hints::inlay_hints_resolve(db, file_id, resolve_range, hash, config, hasher)
    //     })
    // }

    // /// Returns the set of folding ranges.
    // pub fn folding_ranges(&self, file_id: FileId) -> Cancellable<Vec<Fold>> {
    //     self.with_db(|db| {
    //         folding_ranges::folding_ranges(
    //             &db.parse(EditionedFileId::current_edition(file_id)).tree(),
    //         )
    //     })
    // }

    // /// Fuzzy searches for a symbol.
    // pub fn symbol_search(&self, query: Query, limit: usize) -> Cancellable<Vec<NavigationTarget>> {
    //     self.with_db(|db| {
    //         symbol_index::world_symbols(db, query)
    //             .into_iter() // xx: should we make this a par iter?
    //             .filter_map(|s| s.try_to_nav(db))
    //             .take(limit)
    //             .map(UpmappingResult::call_site)
    //             .collect::<Vec<_>>()
    //     })
    // }

    /// Returns the definitions from the symbol at `position`.
    pub fn goto_definition(
        &self,
        position: FilePosition,
    ) -> Cancellable<Option<RangeInfo<NavigationTarget>>> {
        self.with_db(|db| goto_definition::goto_definition(db, position))
    }

    /// Returns the possibly multiple definitions from the symbol at `position`.
    pub fn goto_definition_multi(
        &self,
        position: FilePosition,
    ) -> Cancellable<Option<RangeInfo<Vec<NavigationTarget>>>> {
        self.with_db(|db| goto_definition::goto_definition_multi(db, position))
    }

    // /// Returns the declaration from the symbol at `position`.
    // pub fn goto_declaration(
    //     &self,
    //     position: FilePosition,
    // ) -> Cancellable<Option<RangeInfo<Vec<NavigationTarget>>>> {
    //     self.with_db(|db| goto_declaration::goto_declaration(db, position))
    // }

    // /// Returns the impls from the symbol at `position`.
    // pub fn goto_implementation(
    //     &self,
    //     position: FilePosition,
    // ) -> Cancellable<Option<RangeInfo<Vec<NavigationTarget>>>> {
    //     self.with_db(|db| goto_implementation::goto_implementation(db, position))
    // }

    // /// Returns the type definitions for the symbol at `position`.
    // pub fn goto_type_definition(
    //     &self,
    //     position: FilePosition,
    // ) -> Cancellable<Option<RangeInfo<Vec<NavigationTarget>>>> {
    //     self.with_db(|db| goto_type_definition::goto_type_definition(db, position))
    // }

    /// Finds all usages of the reference at point.
    pub fn find_all_refs(
        &self,
        position: FilePosition,
        search_scope: Option<SearchScope>,
    ) -> Cancellable<Option<ReferenceSearchResult>> {
        self.with_db(|db| references::find_all_refs(db, position, search_scope))
    }

    /// Returns a short text describing element at position.
    pub fn hover(&self, pos: FilePosition) -> Cancellable<Option<RangeInfo<HoverResult>>> {
        self.with_db(|db| hover::hover(db, pos))
    }

    // /// Returns moniker of symbol at position.
    // pub fn moniker(
    //     &self,
    //     position: FilePosition,
    // ) -> Cancellable<Option<RangeInfo<Vec<moniker::MonikerResult>>>> {
    //     self.with_db(|db| moniker::moniker(db, position))
    // }

    // /// Returns URL(s) for the documentation of the symbol under the cursor.
    // /// # Arguments
    // /// * `position` - Position in the file.
    // /// * `target_dir` - Directory where the build output is stored.
    // pub fn external_docs(
    //     &self,
    //     position: FilePosition,
    //     target_dir: Option<&str>,
    //     sysroot: Option<&str>,
    // ) -> Cancellable<doc_links::DocumentationLinks> {
    //     self.with_db(|db| {
    //         doc_links::external_docs(db, position, target_dir, sysroot).unwrap_or_default()
    //     })
    // }

    // /// Computes parameter information at the given position.
    // pub fn signature_help(&self, position: FilePosition) -> Cancellable<Option<SignatureHelp>> {
    //     self.with_db(|db| signature_help::signature_help(db, position))
    // }

    pub fn expr_type_info(&self, position: FilePosition) -> Cancellable<Option<String>> {
        self.with_db(|db| type_info::expr_type_info(db, position))
    }

    // /// Computes call hierarchy candidates for the given file position.
    // pub fn call_hierarchy(
    //     &self,
    //     position: FilePosition,
    // ) -> Cancellable<Option<RangeInfo<Vec<NavigationTarget>>>> {
    //     self.with_db(|db| call_hierarchy::call_hierarchy(db, position))
    // }

    // /// Computes incoming calls for the given file position.
    // pub fn incoming_calls(
    //     &self,
    //     config: CallHierarchyConfig,
    //     position: FilePosition,
    // ) -> Cancellable<Option<Vec<CallItem>>> {
    //     self.with_db(|db| call_hierarchy::incoming_calls(db, config, position))
    // }
    //
    // /// Computes outgoing calls for the given file position.
    // pub fn outgoing_calls(
    //     &self,
    //     config: CallHierarchyConfig,
    //     position: FilePosition,
    // ) -> Cancellable<Option<Vec<CallItem>>> {
    //     self.with_db(|db| call_hierarchy::outgoing_calls(db, config, position))
    // }
    //
    // /// Returns a `mod name;` declaration which created the current module.
    // pub fn parent_module(&self, position: FilePosition) -> Cancellable<Vec<NavigationTarget>> {
    //     self.with_db(|db| parent_module::parent_module(db, position))
    // }

    // /// Returns all transitive reverse dependencies of the given crate,
    // /// including the crate itself.
    // pub fn transitive_rev_deps(&self, crate_id: CrateId) -> Cancellable<Vec<CrateId>> {
    //     self.with_db(|db| db.crate_graph().transitive_rev_deps(crate_id).collect())
    // }

    // /// Returns crates that this file *might* belong to.
    // pub fn relevant_crates_for(&self, file_id: FileId) -> Cancellable<Vec<CrateId>> {
    //     self.with_db(|db| db.relevant_crates(file_id).iter().copied().collect())
    // }
    //
    // /// Returns the edition of the given crate.
    // pub fn crate_edition(&self, crate_id: CrateId) -> Cancellable<Edition> {
    //     self.with_db(|db| db.crate_graph()[crate_id].edition)
    // }

    // /// Returns the root file of the given crate.
    // pub fn crate_root(&self, crate_id: CrateId) -> Cancellable<FileId> {
    //     self.with_db(|db| db.crate_graph()[crate_id].root_file_id)
    // }

    // /// Returns the set of possible targets to run for the current file.
    // pub fn runnables(&self, file_id: FileId) -> Cancellable<Vec<Runnable>> {
    //     self.with_db(|db| runnables::runnables(db, file_id))
    // }
    //
    // /// Returns the set of tests for the given file position.
    // pub fn related_tests(
    //     &self,
    //     position: FilePosition,
    //     search_scope: Option<SearchScope>,
    // ) -> Cancellable<Vec<Runnable>> {
    //     self.with_db(|db| runnables::related_tests(db, position, search_scope))
    // }

    /// Computes syntax highlighting for the given file
    pub fn highlight(&self, file_id: FileId) -> Cancellable<Vec<HlRange>> {
        self.with_db(|db| syntax_highlighting::highlight(db, file_id, None))
    }

    // /// Computes all ranges to highlight for a given item in a file.
    // pub fn highlight_related(
    //     &self,
    //     config: HighlightRelatedConfig,
    //     position: FilePosition,
    // ) -> Cancellable<Option<Vec<HighlightedRange>>> {
    //     self.with_db(|db| {
    //         highlight_related::highlight_related(&Semantics::new(db), config, position)
    //     })
    // }

    /// Computes syntax highlighting for the given file range.
    pub fn highlight_range(&self, frange: FileRange) -> Cancellable<Vec<HlRange>> {
        self.with_db(|db| syntax_highlighting::highlight(db, frange.file_id, Some(frange.range)))
    }

    /// Computes syntax highlighting for the given file.
    pub fn highlight_as_html(&self, file_id: FileId, skip_classes: Vec<String>) -> Cancellable<String> {
        self.with_db(|db| syntax_highlighting::highlight_as_html(db, file_id, skip_classes))
    }

    /// Computes syntax highlighting for the given file without `style {}` prefix.
    pub fn highlight_as_html_no_style(&self, file_id: FileId) -> Cancellable<String> {
        self.with_db(|db| syntax_highlighting::highlight_as_html_no_style(db, file_id))
    }

    /// Computes completions at the given position.
    pub fn completions(
        &self,
        config: &CompletionConfig,
        position: FilePosition,
        trigger_character: Option<char>,
    ) -> Cancellable<Option<Vec<CompletionItem>>> {
        self.with_db(|db| ide_completion::completions(db, config, position, trigger_character))
    }

    // /// Resolves additional completion data at the position given.
    // pub fn resolve_completion_edits(
    //     &self,
    //     config: &CompletionConfig<'_>,
    //     position: FilePosition,
    //     imports: impl IntoIterator<Item = (String, String)> + std::panic::UnwindSafe,
    // ) -> Cancellable<Vec<TextEdit>> {
    //     Ok(self
    //         .with_db(|db| ide_completion::resolve_completion_edits(db, config, position, imports))?
    //         .unwrap_or_default())
    // }

    /// Computes the set of parser level diagnostics for the given file.
    pub fn syntax_diagnostics(
        &self,
        config: &DiagnosticsConfig,
        file_id: FileId,
    ) -> Cancellable<Vec<Diagnostic>> {
        self.with_db(|db| ide_diagnostics::syntax_diagnostics(db, config, file_id))
    }

    /// Computes the set of semantic diagnostics for the given file.
    pub fn semantic_diagnostics(
        &self,
        config: &DiagnosticsConfig,
        resolve: AssistResolveStrategy,
        file_range: FileRange,
    ) -> Cancellable<Vec<Diagnostic>> {
        self.with_db(|db| ide_diagnostics::semantic_diagnostics(db, config, &resolve, file_range))
    }

    /// Computes the set of both syntax and semantic diagnostics for the given file.
    pub fn full_diagnostics(
        &self,
        config: &DiagnosticsConfig,
        resolve: AssistResolveStrategy,
        file_id: FileId,
    ) -> Cancellable<Vec<Diagnostic>> {
        let frange = self.full_file_range(file_id)?;
        self.with_db(|db| ide_diagnostics::full_diagnostics(db, config, &resolve, frange))
    }

    /// Convenience function to return assists + quick fixes for diagnostics
    pub fn assists_with_fixes(
        &self,
        assist_config: &AssistConfig,
        diagnostics_config: &DiagnosticsConfig,
        resolve: AssistResolveStrategy,
        frange: FileRange,
    ) -> Cancellable<Vec<Assist>> {
        let include_fixes = match &assist_config.allowed {
            Some(it) => it.iter().any(|&it| it == AssistKind::QuickFix),
            None => true,
        };
        self.with_db(|db| {
            let diagnostic_assists = if diagnostics_config.enabled && include_fixes {
                ide_diagnostics::semantic_diagnostics(db, diagnostics_config, &resolve, frange)
                    .into_iter()
                    .flat_map(|it| it.fixes.unwrap_or_default())
                    .filter(|it| it.target.intersect(frange.range).is_some())
                    .collect()
            } else {
                Vec::new()
            };
            // let assists = ide_assists::assists(db, assist_config, resolve, frange);

            let res = diagnostic_assists;
            // res.extend(assists);

            res
        })
    }

    /// Returns the edit required to rename reference at the position to the new
    /// name.
    pub fn rename(
        &self,
        position: FilePosition,
        new_name: &str,
    ) -> Cancellable<Result<SourceChange, RenameError>> {
        self.with_db(|db| rename::rename(db, position, new_name))
    }

    pub fn prepare_rename(
        &self,
        position: FilePosition,
    ) -> Cancellable<Result<RangeInfo<()>, RenameError>> {
        self.with_db(|db| rename::prepare_rename(db, position))
    }

    // pub fn will_rename_file(
    //     &self,
    //     file_id: FileId,
    //     new_name_stem: &str,
    // ) -> Cancellable<Option<SourceChange>> {
    //     self.with_db(|db| rename::will_rename_file(db, file_id, new_name_stem))
    // }
    //
    // pub fn structural_search_replace(
    //     &self,
    //     query: &str,
    //     parse_only: bool,
    //     resolve_context: FilePosition,
    //     selections: Vec<FileRange>,
    // ) -> Cancellable<Result<SourceChange, SsrError>> {
    //     self.with_db(|db| {
    //         let rule: ide_ssr::SsrRule = query.parse()?;
    //         let mut match_finder =
    //             ide_ssr::MatchFinder::in_context(db, resolve_context, selections)?;
    //         match_finder.add_rule(rule)?;
    //         let edits = if parse_only { Default::default() } else { match_finder.edits() };
    //         Ok(SourceChange::from_iter(edits))
    //     })
    // }
    //
    // pub fn annotations(
    //     &self,
    //     config: &AnnotationConfig,
    //     file_id: FileId,
    // ) -> Cancellable<Vec<Annotation>> {
    //     self.with_db(|db| annotations::annotations(db, config, file_id))
    // }
    //
    // pub fn resolve_annotation(&self, annotation: Annotation) -> Cancellable<Annotation> {
    //     self.with_db(|db| annotations::resolve_annotation(db, annotation))
    // }
    //

    pub fn file_offset_into_position(&self, file_id: FileId, offset: usize) -> Cancellable<LineCol> {
        self.with_db(|db| root_db::line_index(db, file_id).line_col(TextSize::new(offset as u32)))
    }

    /// Performs an operation on the database that may be canceled.
    ///
    /// rust-analyzer needs to be able to answer semantic questions about the
    /// code while the code is being modified. A common problem is that a
    /// long-running query is being calculated when a new change arrives.
    ///
    /// We can't just apply the change immediately: this will cause the pending
    /// query to see inconsistent state (it will observe an absence of
    /// repeatable read). So what we do is we **cancel** all pending queries
    /// before applying the change.
    ///
    /// Salsa implements cancellation by unwinding with a special value and
    /// catching it on the API boundary.
    fn with_db<F, T>(&self, f: F) -> Cancellable<T>
    where
        F: FnOnce(&RootDatabase) -> T + std::panic::UnwindSafe,
    {
        Cancelled::catch(|| f(&self.db))
    }
}
