use crate::cli::utils;
use camino::Utf8PathBuf;
use clap::Args;
use ide::Analysis;
use ide_db::assists::AssistResolveStrategy;
use ide_diagnostics::config::DiagnosticsConfig;
use paths::AbsPathBuf;
use project_model::DiscoveredManifest;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time;
use std::time::Duration;

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

        self.run_diagnostics_bench(manifest, provided_file_path)?;

        Ok(ExitCode::SUCCESS)
    }

    fn run_diagnostics_bench(
        &self,
        manifest: DiscoveredManifest,
        specific_fpath: AbsPathBuf,
    ) -> anyhow::Result<()> {
        // CPU warm-up
        self.run_bench_once(manifest.clone(), specific_fpath.clone());

        let iterations = self.n_iterations;
        let mut res = vec![];
        for n in 0..iterations {
            println!("iteration: {}", n + 1);
            let elapsed = self.run_bench_once(manifest.clone(), specific_fpath.clone());
            res.push(elapsed);
        }

        println!("{:?}", res);

        let res_average = res.iter().sum::<Duration>() / iterations;
        println!("average = {:?}", res_average);

        Ok(())
    }

    fn run_bench_once(&self, manifest: DiscoveredManifest, specific_fpath: AbsPathBuf) -> Duration {
        let (db, vfs) = utils::init_db(vec![manifest]);

        let all_file_ids = utils::all_roots_file_ids(&db);

        let mut target_file_id = None;
        let analysis = Analysis::new(db);

        for file_id in all_file_ids {
            // // fill parsing cache, we don't want to benchmark those
            let _ = analysis.parse(file_id).unwrap();

            if vfs.file_path(file_id).as_path().unwrap().to_path_buf() == specific_fpath {
                target_file_id = Some(file_id);
            }
        }
        let target_file_id = target_file_id.unwrap();

        let frange = analysis.full_file_range(target_file_id).unwrap();

        let diagnostics_config = DiagnosticsConfig {
            needs_type_annotation: false,
            ..DiagnosticsConfig::test_sample()
        };
        let before = time::Instant::now();
        analysis
            .semantic_diagnostics(&diagnostics_config, AssistResolveStrategy::None, frange)
            .unwrap();
        let elapsed = before.elapsed();

        elapsed
    }
}
