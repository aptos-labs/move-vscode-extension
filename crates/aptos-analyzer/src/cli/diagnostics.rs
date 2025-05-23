use base_db::SourceDatabase;
use base_db::change::FileChanges;
use camino::Utf8PathBuf;
use clap::Args;
use codespan_reporting::diagnostic::{Label, LabelStyle};
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use crossbeam_channel::unbounded;
use ide::AnalysisHost;
use ide_db::assists::AssistResolveStrategy;
use ide_db::{RootDatabase, Severity, root_db};
use ide_diagnostics::config::DiagnosticsConfig;
use ide_diagnostics::diagnostic::Diagnostic;
use lang::builtins_file;
use paths::{AbsPath, AbsPathBuf};
use project_model::aptos_package::{AptosPackage, load_from_fs};
use project_model::project_folders::ProjectFolders;
use project_model::{DiscoveredManifest, dep_graph};
use std::collections::HashSet;
use std::io::stdout;
use std::path::PathBuf;
use std::process::ExitCode;
use stdx::itertools::Itertools;
use vfs::FileId;
use vfs::loader::{Handle, LoadingProgress};

#[derive(Debug, Args)]
pub struct Diagnostics {
    pub path: PathBuf,

    /// Only show diagnostics of kinds (comma separated)
    #[clap(long, value_parser = ["error", "warn", "note"], value_delimiter = ',', num_args=1..)]
    pub kinds: Option<Vec<String>>,

    #[clap(long)]
    pub verbose: bool,
}

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
        let (db, vfs) = load_packages_into_vfs(&all_packages)?;

        let host = AnalysisHost::with_database(db);
        let db = host.raw_database();
        let analysis = host.analysis();

        let mut found_error = false;
        let mut visited_files: HashSet<FileId> = HashSet::default();

        let mut local_package_roots = vec![];
        for package_id in db.all_package_ids().data(db) {
            let package_root = db.package_root(package_id).data(db);
            let root_dir = package_root.root_dir.clone();
            if root_dir.is_some_and(|it| it.starts_with(&ws_root)) && !package_root.is_library {
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
            let package_root_dir = local_package_root.root_dir.as_ref().unwrap();

            if specific_fpath.is_none() {
                println!("processing {package_root_dir}");
            }

            let file_ids = local_package_root.file_set.iter().collect::<Vec<_>>();
            for file_id in file_ids {
                let package_name = local_package_root.root_dir_name().clone().unwrap_or("<error>");
                let file_path = vfs.file_path(file_id);
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
                    if specific_fpath.is_some() || self.verbose {
                        println!(
                            "processing package '{package_name}', file: {}",
                            vfs.file_path(file_id)
                        );
                    }
                    let file_path = vfs.file_path(file_id).as_path().unwrap();
                    for diagnostic in analysis
                        .full_diagnostics(
                            &DiagnosticsConfig::test_sample(),
                            AssistResolveStrategy::None,
                            file_id,
                        )
                        .unwrap()
                    {
                        if let Some(sevs) = diag_kinds.as_ref() {
                            if !sevs.contains(&diagnostic.severity) {
                                continue;
                            }
                        }
                        if matches!(diagnostic.severity, Severity::Error) {
                            found_error = true;
                        }
                        print_diagnostic(db, file_path, diagnostic);
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

fn print_diagnostic(db: &RootDatabase, file_path: &AbsPath, diagnostic: Diagnostic) {
    let Diagnostic {
        code,
        message,
        range,
        severity,
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
    let file_text = db.file_text(range.file_id).text(db);

    let mut files = codespan_reporting::files::SimpleFiles::new();
    let file_id = files.add(file_path.to_string(), file_text);

    let diagnostic = codespan_reporting::diagnostic::Diagnostic::new(severity)
        .with_label(Label::new(LabelStyle::Primary, file_id, range.range))
        .with_code(code.as_str())
        .with_message(message);

    let term_config = term::Config::default();
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    term::emit(&mut stderr, &term_config, &files, &diagnostic).unwrap();
}

fn load_packages_into_vfs(packages: &[AptosPackage]) -> anyhow::Result<(RootDatabase, vfs::Vfs)> {
    let (sender, receiver) = unbounded();
    let mut vfs = vfs::Vfs::default();
    let mut vfs_loader = {
        let loader = vfs_notify::NotifyHandle::spawn(sender);
        Box::new(loader)
    };

    let package_graph = dep_graph::collect(&packages, &mut |path: &AbsPath| {
        let contents = vfs_loader.load_sync(path);
        let path = vfs::VfsPath::from(path.to_path_buf());
        vfs.set_file_contents(path.clone(), contents);
        vfs.file_id(&path)
            .and_then(|(file_id, excluded)| (excluded == vfs::FileExcluded::No).then_some(file_id))
    });

    let project_folders = ProjectFolders::new(&packages);
    // sends `vfs::loader::message::Loaded { files }` events for project folders
    vfs_loader.set_config(vfs::loader::Config {
        load: project_folders.load,
        watch: vec![],
        version: 0,
    });

    let mut db = RootDatabase::new();
    let mut analysis_change = FileChanges::new();

    // wait until Vfs has loaded all roots
    for task in receiver {
        match task {
            vfs::loader::Message::Progress { n_done, .. } => {
                if n_done == LoadingProgress::Finished {
                    break;
                }
            }
            vfs::loader::Message::Loaded { files } | vfs::loader::Message::Changed { files } => {
                let _p = tracing::info_span!("load_cargo::load_crate_craph/LoadedChanged").entered();
                for (path, contents) in files {
                    vfs.set_file_contents(path.into(), contents);
                }
            }
        }
    }
    let changes = vfs.take_changes();
    for (_, file) in changes {
        if let vfs::Change::Create(v, _) | vfs::Change::Modify(v, _) = file.change {
            if let Ok(text) = String::from_utf8(v) {
                analysis_change.change_file(file.file_id, Some(text))
            }
        }
    }
    let package_root_config = project_folders.package_root_config;
    let package_roots = package_root_config.partition_into_package_roots(&vfs);
    analysis_change.set_package_roots(package_roots);

    analysis_change.set_package_graph(package_graph.unwrap_or_default());

    db.apply_change(analysis_change);

    let builtins_change = builtins_file::add_to_vfs(&mut vfs);
    db.apply_change(builtins_change);

    Ok((db, vfs))
}
