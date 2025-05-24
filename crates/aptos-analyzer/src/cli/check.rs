use base_db::SourceDatabase;
use base_db::change::FileChanges;
use base_db::package_root::PackageKind;
use camino::Utf8PathBuf;
use clap::Args;
use codespan_reporting::diagnostic::{Label, LabelStyle};
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use ide::Analysis;
use ide_db::assists::{Assist, AssistResolveStrategy};
use ide_db::{RootDatabase, Severity};
use ide_diagnostics::config::DiagnosticsConfig;
use ide_diagnostics::diagnostic::Diagnostic;
use paths::{AbsPath, AbsPathBuf};
use project_model::DiscoveredManifest;
use project_model::aptos_package::load_from_fs;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use stdx::itertools::Itertools;
use syntax::TextRange;
use vfs::FileId;
use vfs::loader::Handle;

#[derive(Debug, Args)]
pub struct Check {
    pub path: PathBuf,

    /// Only show diagnostics of kinds (comma separated)
    #[clap(long, value_parser = ["error", "warn", "note"], value_delimiter = ',', num_args=1..)]
    pub kinds: Option<Vec<String>>,

    #[clap(long)]
    pub verbose: bool,

    #[clap(long)]
    pub fix: bool,
}

impl Check {
    pub fn run(self) -> anyhow::Result<ExitCode> {
        const STACK_SIZE: usize = 1024 * 1024 * 8;

        let handle =
            stdx::thread::Builder::new(stdx::thread::ThreadIntent::LatencySensitive, "BIG_STACK_THREAD")
                .stack_size(STACK_SIZE)
                .spawn(|| self.run_())
                .unwrap();

        handle.join()
    }

    fn run_(self) -> anyhow::Result<ExitCode> {
        let provided_path =
            Utf8PathBuf::from_path_buf(std::env::current_dir()?.join(&self.path)).unwrap();

        let mut specific_fpath = None;
        let mut ws_root = None;
        let manifests = if provided_path.is_file() && provided_path.extension() == Some("move") {
            let abs_path = AbsPathBuf::assert(provided_path);
            let manifest = DiscoveredManifest::discover_for_file(&abs_path);
            specific_fpath = Some(abs_path);
            manifest
                .map(|it| {
                    ws_root = Some(it.content_root());
                    vec![it]
                })
                .unwrap_or_default()
        } else {
            let provided_ws_root = AbsPathBuf::assert(provided_path);
            let manifests = DiscoveredManifest::discover_all(&[provided_ws_root.clone()]);
            ws_root = Some(provided_ws_root);
            manifests
        };

        if manifests.is_empty() {
            eprintln!("Could not find any Aptos packages.");
            return Ok(ExitCode::FAILURE);
        }
        let ws_root = ws_root.unwrap();

        self.run_diagnostics(manifests, ws_root, specific_fpath)
    }

    fn run_diagnostics(
        &self,
        ws_manifests: Vec<DiscoveredManifest>,
        ws_root: AbsPathBuf,
        specific_fpath: Option<AbsPathBuf>,
    ) -> anyhow::Result<ExitCode> {
        let all_packages = load_from_fs::load_aptos_packages(ws_manifests)
            .into_iter()
            .filter_map(|it| it.ok())
            .collect::<Vec<_>>();
        let (mut db, mut vfs) = ide_db::load::load_db(&all_packages)?;

        let mut found_error = false;
        let mut visited_files: HashSet<FileId> = HashSet::default();

        let mut local_package_roots = vec![];
        for package_id in db.all_package_ids().data(&db) {
            let package_root = db.package_root(package_id).data(&db);
            let root_dir = package_root.root_dir(&vfs).clone();
            if root_dir.is_some_and(|it| it.starts_with(&ws_root)) && !package_root.is_library() {
                local_package_roots.push(package_root);
            }
        }

        let diag_kinds = self.kinds.clone().map(|it| {
            it.iter()
                .map(|severity| match severity.as_str() {
                    "error" => Severity::Error,
                    "warn" => Severity::Warning,
                    "note" => Severity::WeakWarning,
                    _ => unreachable!(),
                })
                .collect::<Vec<_>>()
        });

        for local_package_root in local_package_roots {
            let package_root_dir = local_package_root.root_dir(&vfs).unwrap();

            if specific_fpath.is_none() {
                println!("processing {package_root_dir}");
            }

            let file_ids = local_package_root.file_set.iter().collect::<Vec<_>>();
            for file_id in file_ids {
                let file_path = vfs.file_path(file_id).clone();
                if !file_path
                    .name_and_extension()
                    .is_some_and(|(name, ext)| ext == Some("move"))
                {
                    if self.verbose {
                        println!("skip file {}", file_path);
                    }
                    visited_files.insert(file_id);
                    continue;
                }

                // skipping all files except for `specific_fpath` if set
                if let Some(specific_fpath) = specific_fpath.as_ref() {
                    if file_path.as_path().unwrap().to_path_buf() != specific_fpath {
                        continue;
                    }
                }
                if !visited_files.contains(&file_id) {
                    let package_name = local_package_root
                        .root_dir_name(&vfs)
                        .unwrap_or("<error>".to_string());
                    if specific_fpath.is_some() || self.verbose {
                        println!(
                            "processing package '{package_name}', file: {}",
                            vfs.file_path(file_id)
                        );
                    }
                    let abs_file_path = vfs.file_path(file_id).as_path().unwrap().to_path_buf();

                    let diagnostics = find_diagnostics_for_a_file(&db, file_id, &diag_kinds);
                    let file_text = db.file_text(file_id).text(&db);
                    if !self.fix {
                        for diagnostic in diagnostics.clone() {
                            if diagnostic.severity == Severity::Error {
                                found_error = true;
                            }
                            print_diagnostic(&file_text, &abs_file_path, diagnostic, false);
                        }
                    }

                    let mut diagnostics_with_fixes = diagnostics
                        .into_iter()
                        .filter(|diag| diag.fixes.as_ref().is_some_and(|it| !it.is_empty()))
                        .collect::<Vec<_>>();
                    if self.fix && !diagnostics_with_fixes.is_empty() {
                        let mut file_text = file_text.to_string();
                        loop {
                            match apply_first_fix(
                                &file_text,
                                abs_file_path.as_path(),
                                diagnostics_with_fixes,
                            ) {
                                Some(new_file_text) => {
                                    let mut change = FileChanges::new();
                                    change.change_file(file_id, Some(new_file_text.clone()));
                                    db.apply_change(change);

                                    vfs.set_file_contents(
                                        file_path.to_owned(),
                                        Some(new_file_text.clone().into_bytes()),
                                    );
                                    fs::write(&abs_file_path, new_file_text.clone())?;
                                    file_text = new_file_text;
                                }
                                None => {
                                    break;
                                }
                            }
                            diagnostics_with_fixes =
                                find_diagnostics_for_a_file(&db, file_id, &diag_kinds);
                        }
                    }
                }

                visited_files.insert(file_id);
            }
        }

        if self.verbose {
            println!();
            println!("diagnostic scan complete");
        }

        let mut exit_code = ExitCode::SUCCESS;
        if found_error {
            println!();
            println!("Error: diagnostic error detected");
            exit_code = ExitCode::FAILURE;
        }

        Ok(exit_code)
    }
}

fn find_diagnostics_for_a_file(
    db: &RootDatabase,
    file_id: FileId,
    diag_kinds: &Option<Vec<Severity>>,
) -> Vec<Diagnostic> {
    let analysis = Analysis::new(db.snapshot());
    let mut diagnostics = analysis
        .full_diagnostics(
            &DiagnosticsConfig::test_sample(),
            AssistResolveStrategy::None,
            file_id,
        )
        .unwrap();
    if let Some(sevs) = diag_kinds {
        diagnostics = diagnostics
            .into_iter()
            .filter(|it| sevs.contains(&it.severity))
            .collect();
    }
    diagnostics
}

fn apply_first_fix(
    file_text: &str,
    file_path: &AbsPath,
    diagnostics: Vec<Diagnostic>,
) -> Option<String> {
    for diagnostic in diagnostics {
        let fixes = diagnostic.fixes.clone().unwrap_or_default();
        if !fixes.is_empty() {
            print_diagnostic(file_text, file_path, diagnostic, true);
            let fix = fixes.first().unwrap();
            let (new_file_text, _) = apply_fix(fix, file_text.as_ref());
            return Some(new_file_text);
        }
    }
    None
}

fn apply_fix(fix: &Assist, before: &str) -> (String, Vec<TextRange>) {
    let source_change = fix.source_change.as_ref().unwrap();
    let mut after = before.to_string();
    let mut new_text_ranges = vec![];
    for (text_edit, snippet_edit) in source_change.source_file_edits.values() {
        new_text_ranges.extend(text_edit.iter().map(|it| it.new_range()));
        text_edit.apply(&mut after);
        if let Some(snippet_edit) = snippet_edit {
            snippet_edit.apply(&mut after);
        }
    }

    (after, new_text_ranges)
}

fn print_diagnostic(file_text: &str, file_path: &AbsPath, diagnostic: Diagnostic, show_fix: bool) {
    let Diagnostic {
        code,
        message,
        range,
        severity,
        fixes,
        ..
    } = diagnostic;

    let severity = match severity {
        Severity::Error => codespan_reporting::diagnostic::Severity::Error,
        Severity::Warning => codespan_reporting::diagnostic::Severity::Warning,
        Severity::WeakWarning => codespan_reporting::diagnostic::Severity::Note,
        _ => {
            return;
        }
    };

    let mut files = codespan_reporting::files::SimpleFiles::new();
    let file_id = files.add(file_path.to_string(), file_text.to_string());

    let mut codespan_diagnostic = codespan_reporting::diagnostic::Diagnostic::new(severity)
        .with_label(Label::new(LabelStyle::Primary, file_id, range.range))
        .with_code(code.as_str())
        .with_message(message);

    if show_fix {
        let fixes = fixes.unwrap_or_default();
        if let Some(fix) = fixes.first() {
            let (new_file_text, new_file_ranges) = apply_fix(fix, &file_text);
            let file_id = files.add(file_path.to_string(), new_file_text);
            for new_file_range in new_file_ranges {
                codespan_diagnostic = codespan_diagnostic.with_label(
                    Label::new(LabelStyle::Primary, file_id, new_file_range).with_message("after fix"),
                )
            }
        }
    }

    let term_config = term::Config::default();
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    term::emit(&mut stderr, &term_config, &files, &codespan_diagnostic).unwrap();
}
