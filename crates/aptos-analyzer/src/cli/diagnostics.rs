use base_db::SourceDatabase;
use base_db::change::FileChanges;
use clap::{Args, Parser};
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
use std::path::PathBuf;
use std::process::ExitCode;
use stdx::itertools::Itertools;
use vfs::FileId;
use vfs::loader::{Handle, LoadingProgress};

#[derive(Debug, Args)]
pub struct Diagnostics {
    pub path: PathBuf,
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
        let ws_root = AbsPathBuf::assert_utf8(std::env::current_dir()?.join(&self.path));
        let manifests = DiscoveredManifest::discover_all(&[ws_root.to_path_buf()]);

        let all_packages = load_from_fs::load_aptos_packages(manifests)
            .into_iter()
            .filter_map(|it| it.ok())
            .collect::<Vec<_>>();

        let (db, mut vfs) = load_packages_into_vfs(&all_packages)?;

        let host = AnalysisHost::with_database(db);
        let db = host.raw_database();
        let analysis = host.analysis();

        let mut found_error = false;
        let mut visited_files: HashSet<FileId> = HashSet::default();

        let mut local_package_roots = vec![];
        for package_id in db.all_package_ids().data(db) {
            let package_root = db.package_root(package_id).data(db);
            if !package_root.is_library {
                local_package_roots.push(package_root);
            }
        }

        for local_package_root in local_package_roots {
            let file_ids = local_package_root.file_set.iter().collect::<Vec<_>>();
            for file_id in file_ids {
                let package_name = local_package_root
                    .root_dir
                    .clone()
                    .unwrap_or("<error>".to_string());
                let file_path = vfs.file_path(file_id);
                if !file_path
                    .name_and_extension()
                    .is_some_and(|(name, ext)| ext == Some("move"))
                {
                    println!("skip file {}", file_path);
                    visited_files.insert(file_id);
                    continue;
                }
                if !visited_files.contains(&file_id) {
                    println!(
                        "processing package '{package_name}', file: {}",
                        vfs.file_path(file_id)
                    );
                    for diagnostic in analysis
                        .full_diagnostics(
                            &DiagnosticsConfig::test_sample(),
                            AssistResolveStrategy::None,
                            file_id,
                        )
                        .unwrap()
                    {
                        if matches!(diagnostic.severity, Severity::Error) {
                            found_error = true;
                        }
                        print_diagnostic(db, diagnostic);
                    }
                }

                visited_files.insert(file_id);
            }
        }

        println!();
        println!("diagnostic scan complete");

        let mut exit_code = ExitCode::SUCCESS;
        if found_error {
            println!();
            println!("Error: diagnostic error detected");
            exit_code = ExitCode::FAILURE;
        }

        Ok(exit_code)
    }
}

fn print_diagnostic(db: &RootDatabase, diagnostic: Diagnostic) {
    let Diagnostic {
        code,
        message,
        range,
        severity,
        ..
    } = diagnostic;
    let line_index = root_db::line_index(db, range.file_id);
    let start = line_index.line_col(range.range.start());
    let end = line_index.line_col(range.range.end());
    println!("{severity:?} {code:?} from {start:?} to {end:?}: {message}");
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
