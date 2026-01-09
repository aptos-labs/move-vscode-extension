// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::cli::utils;
use crate::cli::utils::{CmdPath, CmdPathKind};
use base_db::SourceDatabase;
use clap::Args;
use codespan_reporting::diagnostic::{Label, LabelStyle};
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use ide::Analysis;
use ide_db::assist_config::AssistConfig;
use ide_db::assists::{Assist, AssistResolveStrategy};
use ide_db::{RootDatabase, Severity};
use ide_diagnostics::config::DiagnosticsConfig;
use ide_diagnostics::diagnostic::Diagnostic;
use paths::{AbsPath, AbsPathBuf, RelPathBuf};
use project_model::DiscoveredManifest;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::ExitCode;
use vfs::FileId;

#[derive(Debug, Args)]
pub struct Diagnostics {
    pub path: PathBuf,

    /// Only show diagnostics of kinds (comma separated)
    #[clap(long, value_parser = ["error", "warn", "note"], value_delimiter = ',', num_args=1..)]
    pub kinds: Option<Vec<String>>,

    /// Codes for disabled diagnostics (comma separated)
    #[clap(long, value_delimiter = ',', num_args=1..)]
    pub disable: Option<Vec<String>>,

    /// Disable all diagnostics, except for ones provided by this flag (comma separated)
    #[clap(long, value_delimiter = ',', num_args=1..)]
    pub enable_only: Option<Vec<String>>,

    #[clap(long)]
    pub verbose: bool,

    #[clap(short, long)]
    pub quiet: bool,

    /// Codes for quickfixes to apply (comma separated). Specify 'all' as value to apply everything.
    #[clap(long, value_delimiter = ',', num_args=1..)]
    pub apply_fixes: Option<Vec<String>>,
}

const FORBIDDEN_PATH_SUFFIXES: &[&str] = &["move-examples/scripts/too_large"];

impl Diagnostics {
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
        let cmd_path = CmdPath::new(&self.path)?;
        match cmd_path.kind() {
            CmdPathKind::Workspace(ws_root) => self.run_diagnostics_for_ws_root(ws_root),
            CmdPathKind::MoveFile(file_path) => self.run_diagnostics_for_single_file(file_path),
            _ => {
                eprintln!("Provide either Move file or directory.");
                Ok(ExitCode::FAILURE)
            }
        }
    }

    fn run_diagnostics_for_ws_root(&self, ws_root: AbsPathBuf) -> anyhow::Result<ExitCode> {
        let ws_manifests = DiscoveredManifest::discover_all(&[ws_root.clone()]);
        if ws_manifests.is_empty() {
            eprintln!("Could not find any Aptos packages.");
            return Ok(ExitCode::FAILURE);
        }

        let (mut db, mut vfs) = utils::init_db(ws_manifests);

        let cmd_config = self.prepare_cmd_config();
        let ws_package_roots = utils::ws_package_roots(&db, &vfs, ws_root);

        let mut found_error = false;
        let mut visited_files: HashSet<FileId> = HashSet::default();

        for ws_package_root in ws_package_roots {
            let manifest_file_id = ws_package_root.manifest_file_id.unwrap();
            let package_metadata = db.package_metadata(manifest_file_id).metadata(&db);

            let package_root_dir = ws_package_root.root_dir(&vfs).unwrap();
            if FORBIDDEN_PATH_SUFFIXES.iter().any(|path_suffix| {
                let suffix = RelPathBuf::try_from(*path_suffix).unwrap();
                package_root_dir.ends_with(suffix.as_path())
            }) {
                println!("skip {package_root_dir} [forbidden]");
                continue;
            }

            if !self.quiet {
                print!("processing {package_root_dir}");
                if !package_metadata.resolve_deps {
                    print!(" [no_deps]");
                }
                println!()
            }

            let package_cmd_config = cmd_config
                .clone()
                .with_resolve_deps(package_metadata.resolve_deps);

            for file_id in ws_package_root.file_ids() {
                let file_path = vfs.file_path(file_id).clone();
                if !file_path
                    .name_and_extension()
                    .is_some_and(|(_, ext)| ext == Some("move"))
                {
                    if self.verbose {
                        println!("skip file {}", file_path);
                    }
                    visited_files.insert(file_id);
                    continue;
                }

                if !visited_files.contains(&file_id) {
                    let package_name = ws_package_root
                        .root_dir_name(&vfs)
                        .unwrap_or("<error>".to_string());
                    if self.verbose {
                        println!(
                            "processing package '{package_name}', file: {}",
                            vfs.file_path(file_id)
                        );
                    }
                    let abs_file_path = vfs.file_path(file_id).as_path().unwrap().to_path_buf();
                    if self.verbose {
                        println!("{}", abs_file_path);
                    }

                    let apply_assists = package_cmd_config.allowed_fix_codes.has_codes_to_apply();
                    if !apply_assists {
                        found_error = found_error
                            || self.print_diagnostics_for_a_file(
                                &db,
                                &package_cmd_config,
                                file_id,
                                &abs_file_path,
                            );
                        continue;
                    }

                    self.apply_all_diagnostic_fixes(
                        &mut db,
                        &mut vfs,
                        &cmd_config,
                        file_id,
                        &abs_file_path,
                    );
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

    fn run_diagnostics_for_single_file(&self, target_fpath: AbsPathBuf) -> anyhow::Result<ExitCode> {
        println!("Searching for a closest Move.toml...");
        let manifest = DiscoveredManifest::discover_for_file(&target_fpath)
            .expect("file does not belong to a package");
        println!("Found `{}`", manifest.move_toml_file);

        let (mut db, mut vfs) = utils::init_db(vec![manifest]);

        let cmd_config = self.prepare_cmd_config();
        let target_file_id = utils::find_target_file_id(&db, &vfs, target_fpath.clone())
            .expect(&format!("cannot find file `{}` in VFS", target_fpath.clone()));

        let mut found_error = false;
        let apply_assists = cmd_config.allowed_fix_codes.has_codes_to_apply();
        if !apply_assists {
            found_error = found_error
                || self.print_diagnostics_for_a_file(&db, &cmd_config, target_file_id, &target_fpath);
        } else {
            self.apply_all_diagnostic_fixes(
                &mut db,
                &mut vfs,
                &cmd_config,
                target_file_id,
                &target_fpath,
            );
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

    fn print_diagnostics_for_a_file(
        &self,
        db: &RootDatabase,
        cmd_config: &CmdConfig,
        file_id: FileId,
        file_path: &AbsPath,
    ) -> bool {
        let file_text = db.file_text(file_id).text(db).to_string();
        let diagnostics = find_diagnostics_for_a_file(db, file_id, &cmd_config);
        let mut found_error = false;
        for diagnostic in diagnostics.clone() {
            if diagnostic.severity == Severity::Error {
                found_error = true;
            }
            print_diagnostic(&file_text, &file_path, diagnostic, false);
        }
        found_error
    }

    fn apply_all_diagnostic_fixes(
        &self,
        db: &mut RootDatabase,
        vfs: &mut vfs::Vfs,
        cmd_config: &CmdConfig,
        file_id: FileId,
        file_path: &AbsPath,
    ) {
        // let apply_assists = cmd_config.allowed_fix_codes.has_codes_to_apply();
        let mut diagnostics = find_diagnostics_for_a_file(db, file_id, &cmd_config);

        let mut current_file_text = db.file_text(file_id).text(db).to_string();
        loop {
            match find_diagnostic_with_fixes(db, diagnostics, &cmd_config) {
                Some((diagnostic, fix)) => {
                    print_diagnostic(&current_file_text, file_path, diagnostic, true);
                    (current_file_text, _) = utils::apply_assist(&fix, current_file_text.as_ref());
                    utils::write_file_text(db, vfs, file_id, &current_file_text);
                }
                None => break,
            }
            diagnostics = find_diagnostics_for_a_file(&db, file_id, &cmd_config);
        }
    }

    fn prepare_cmd_config(&self) -> CmdConfig {
        let mut diagnostics_config = DiagnosticsConfig::test_sample();

        let enable_only = self.enable_only.clone().unwrap_or_default();
        if !enable_only.is_empty() {
            println!("enabled diagnostics: {:?}", enable_only);
            diagnostics_config.enable_only = enable_only.into_iter().collect();
        } else {
            let disabled_codes = self.disable.clone().unwrap_or_default();
            println!("disabled diagnostics: {:?}", disabled_codes);
            diagnostics_config.disabled = disabled_codes.into_iter().collect();
        }

        let fix_codes = FixCodes::from_cli(self.apply_fixes.as_ref());
        if fix_codes != FixCodes::None {
            diagnostics_config = diagnostics_config.for_assists();
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

        CmdConfig {
            diagnostics_config,
            allowed_fix_codes: fix_codes,
            kinds: diag_kinds,
        }
    }
}

#[derive(Debug, Clone)]
struct CmdConfig {
    diagnostics_config: DiagnosticsConfig,
    kinds: Option<Vec<Severity>>,
    allowed_fix_codes: FixCodes,
}

impl CmdConfig {
    pub fn with_resolve_deps(mut self, resolve_deps: bool) -> Self {
        if !resolve_deps {
            // disables most of the diagnostics
            self.diagnostics_config = self.diagnostics_config.for_assists();
        }
        self
    }
}

fn find_diagnostics_for_a_file(
    db: &RootDatabase,
    file_id: FileId,
    cmd_config: &CmdConfig,
) -> Vec<Diagnostic> {
    let analysis = Analysis::new(db.snapshot());
    let mut diagnostics = analysis
        .full_diagnostics(
            &cmd_config.diagnostics_config,
            AssistResolveStrategy::None,
            file_id,
        )
        .unwrap();
    if let Some(sevs) = &cmd_config.kinds {
        diagnostics = diagnostics
            .into_iter()
            .filter(|it| sevs.contains(&it.severity))
            .collect();
    }
    diagnostics
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
            let (new_file_text, new_file_ranges) = utils::apply_assist(fix, &file_text);
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub(super) enum FixCodes {
    None,
    Codes(Vec<String>),
    All,
}

impl FixCodes {
    pub(super) fn from_cli(apply_fixes: Option<&Vec<String>>) -> Self {
        match apply_fixes {
            None => FixCodes::None,
            Some(codes) if codes.contains(&"all".to_string()) => FixCodes::All,
            Some(codes) => FixCodes::Codes(codes.clone()),
        }
    }

    pub(super) fn has_codes_to_apply(&self) -> bool {
        matches!(self, FixCodes::Codes(_) | FixCodes::All)
    }
}

fn find_diagnostic_with_fixes(
    db: &RootDatabase,
    diagnostics: Vec<Diagnostic>,
    cmd_config: &CmdConfig,
) -> Option<(Diagnostic, Assist)> {
    let analysis = Analysis::new(db.snapshot());
    for mut diagnostic in diagnostics {
        if diagnostic.fixes.unwrap_or_default().is_empty() {
            continue;
        }
        let resolved_fixes = analysis
            .assists_with_fixes(
                &AssistConfig { allowed: None },
                &cmd_config.diagnostics_config.clone().for_assists(),
                AssistResolveStrategy::All,
                diagnostic.range,
            )
            .unwrap();
        diagnostic.fixes = Some(resolved_fixes.clone());
        for fix in resolved_fixes {
            match &cmd_config.allowed_fix_codes {
                FixCodes::All => return Some((diagnostic, fix)),
                FixCodes::Codes(allowed_codes) => {
                    if allowed_codes.contains(&fix.id.0.to_string()) {
                        return Some((diagnostic, fix));
                    }
                }
                FixCodes::None => unreachable!(),
            }
        }
    }
    None
}
