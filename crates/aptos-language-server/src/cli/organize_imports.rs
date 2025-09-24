use crate::cli::utils;
use base_db::SourceDatabase;
use camino::Utf8PathBuf;
use clap::Args;
use ide::Analysis;
use ide_db::RootDatabase;
use paths::AbsPathBuf;
use project_model::DiscoveredManifest;
use std::path::PathBuf;
use std::process::ExitCode;
use vfs::{FileId, Vfs};

#[derive(Debug, Args)]
pub struct OrganizeImports {
    /// Path to a Move file.
    pub path: PathBuf,
}

impl OrganizeImports {
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

        let (mut db, mut vfs) = utils::init_db(vec![manifest]);

        let all_file_ids = utils::all_roots_file_ids(&db);
        for file_id in all_file_ids {
            self.organize_imports_in_file(&mut db, &mut vfs, file_id);
        }
        // let target_file_id = utils::find_target_file_id(&db, &vfs, provided_file_path).unwrap();
        // self.organize_imports_in_file(&mut db, &mut vfs, file_id);

        Ok(ExitCode::SUCCESS)
    }

    fn organize_imports_in_file(&self, db: &mut RootDatabase, vfs: &mut Vfs, target_file_id: FileId) {
        let file_text = db.file_text(target_file_id).text(db).to_string();

        let organize_imports_assist = {
            let analysis = Analysis::new(db.snapshot());
            analysis.organize_imports(target_file_id).unwrap().unwrap()
        };

        let (new_file_text, _) = utils::apply_assist(&organize_imports_assist, file_text.as_ref());
        if new_file_text != file_text {
            if let Some(fpath) = vfs.file_path(target_file_id).as_path() {
                println!("organizing imports in {}", fpath);
            }
            utils::write_file_text(db, vfs, target_file_id, &new_file_text);
        }
    }
}
