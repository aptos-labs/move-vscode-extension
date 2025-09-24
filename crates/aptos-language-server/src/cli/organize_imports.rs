use crate::cli::utils;
use crate::cli::utils::{CmdPath, CmdPathKind};
use base_db::SourceDatabase;
use clap::Args;
use ide::Analysis;
use ide_db::RootDatabase;
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
        let cmd_path = CmdPath::new(&self.path)?;
        match cmd_path.kind() {
            CmdPathKind::MoveFile(target_fpath) => {
                let manifest = DiscoveredManifest::discover_for_file(&target_fpath)
                    .expect("cannot find manifest for provided path");
                let (mut db, mut vfs) = utils::init_db(vec![manifest]);

                let target_file_id = utils::find_target_file_id(&db, &vfs, target_fpath).unwrap();

                self.organize_imports_in_file(&mut db, &mut vfs, target_file_id);
            }
            CmdPathKind::Workspace(ws_root) => {
                let ws_manifests = DiscoveredManifest::discover_all(&[ws_root.clone()]);
                if ws_manifests.is_empty() {
                    eprintln!("Could not find any Aptos packages.");
                    return Ok(ExitCode::FAILURE);
                }
                let (mut db, mut vfs) = utils::init_db(ws_manifests);
                let ws_package_roots = utils::ws_package_roots(&db, &vfs, ws_root);
                for ws_package_root in ws_package_roots {
                    for file_id in ws_package_root.file_ids() {
                        self.organize_imports_in_file(&mut db, &mut vfs, file_id);
                    }
                }
            }
            _ => (),
        }

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
