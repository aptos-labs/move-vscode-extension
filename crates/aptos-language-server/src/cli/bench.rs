use base_db::SourceDatabase;
use camino::Utf8PathBuf;
use clap::Args;
use ide::Analysis;
use ide_db::assists::AssistResolveStrategy;
use ide_diagnostics::config::DiagnosticsConfig;
use paths::AbsPathBuf;
use project_model::DiscoveredManifest;
use project_model::aptos_package::load_from_fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;
use std::{fs, time};

#[derive(Debug, Args)]
pub struct Bench {
    /// Path to a Move file for which compute diagnostics.
    pub path: PathBuf,

    #[clap(short, long, default_value_t = 5)]
    pub n_iterations: u32,
}

impl Bench {
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

        let provided_file_path = AbsPathBuf::assert(provided_path);
        let manifest = DiscoveredManifest::discover_for_file(&provided_file_path)
            .expect("cannot find manifest for provided path");
        let ws_root = manifest.content_root();

        self.run_diagnostics_bench(manifest, ws_root, provided_file_path)?;

        Ok(ExitCode::SUCCESS)
    }

    fn run_diagnostics_bench(
        &self,
        manifest: DiscoveredManifest,
        ws_root: AbsPathBuf,
        specific_fpath: AbsPathBuf,
    ) -> anyhow::Result<()> {
        let canonical_ws_root = AbsPathBuf::assert_utf8(fs::canonicalize(ws_root.clone())?);
        // CPU warm-up
        self.run_bench_once(
            manifest.clone(),
            canonical_ws_root.clone(),
            specific_fpath.clone(),
        );
        let iterations = self.n_iterations;
        let mut res = vec![];
        for n in 0..iterations {
            println!("iteration: {n}");
            let elapsed = self.run_bench_once(
                manifest.clone(),
                canonical_ws_root.clone(),
                specific_fpath.clone(),
            );
            res.push(elapsed);
        }

        println!("{:?}", res);

        let res_average = res.iter().sum::<Duration>() / iterations;
        println!("average = {:?}", res_average);

        Ok(())
    }

    fn run_bench_once(
        &self,
        manifest: DiscoveredManifest,
        canonical_ws_root: AbsPathBuf,
        specific_fpath: AbsPathBuf,
    ) -> Duration {
        let diagnostics_config = DiagnosticsConfig {
            needs_type_annotation: false,
            ..DiagnosticsConfig::test_sample()
        };
        let aptos_packages = load_from_fs::load_aptos_packages(vec![manifest]).valid_packages();

        let (db, vfs) = ide_db::load::load_db(&aptos_packages).unwrap();

        let mut local_package_roots = vec![];
        for package_id in db.all_package_ids().data(&db) {
            let package_root = db.package_root(package_id).data(&db);
            if package_root.is_builtin() {
                continue;
            }
            let root_dir = package_root.root_dir(&vfs).clone();
            if root_dir.is_some_and(|it| it.starts_with(&canonical_ws_root))
                && !package_root.is_library()
            {
                local_package_roots.push(package_root);
            }
        }

        let analysis = Analysis::new(db);

        let mut target_file_id = None;
        for local_package_root in local_package_roots {
            let file_ids = local_package_root.file_set.iter();
            for file_id in file_ids {
                let file_path = vfs.file_path(file_id);

                // // fill parsing cache, we don't want to benchmark those
                // let _ = analysis.parse(file_id).unwrap();

                if file_path.as_path().unwrap().to_path_buf() == specific_fpath {
                    target_file_id = Some(file_id);
                }
            }
        }
        let target_file_id = target_file_id.unwrap();
        let frange = analysis.full_file_range(target_file_id).unwrap();

        let before = time::Instant::now();
        analysis
            .semantic_diagnostics(&diagnostics_config, AssistResolveStrategy::None, frange)
            .unwrap();
        let elapsed = before.elapsed();

        elapsed
    }
}
