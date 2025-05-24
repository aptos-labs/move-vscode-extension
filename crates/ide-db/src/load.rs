use crate::RootDatabase;
use base_db::change::FileChanges;
use crossbeam_channel::unbounded;
use lang::builtins_file;
use project_model::aptos_package::AptosPackage;
use project_model::dep_graph;
use project_model::dep_graph::collect_initial;
use project_model::project_folders::ProjectFolders;
use vfs::AbsPath;
use vfs::loader::{Handle, LoadingProgress};

pub fn load_db(packages: &[AptosPackage]) -> anyhow::Result<(RootDatabase, vfs::Vfs)> {
    let (sender, receiver) = unbounded();
    let mut vfs = vfs::Vfs::default();
    let mut vfs_loader = {
        let loader = vfs_notify::NotifyHandle::spawn(sender);
        Box::new(loader)
    };

    let mut db = RootDatabase::new();

    let package_graph = collect_initial(&packages, &mut vfs);
    let mut set_graph = FileChanges::new();
    set_graph.set_package_graph(package_graph.unwrap_or_default());
    db.apply_change(set_graph);

    let project_folders = ProjectFolders::new(&packages);
    // sends `vfs::loader::message::Loaded { files }` events for project folders
    vfs_loader.set_config(vfs::loader::Config {
        load: project_folders.load,
        watch: vec![],
        version: 0,
    });

    let mut analysis_change = FileChanges::new();

    // wait until Vfs has loaded all roots
    for task in receiver {
        match task {
            vfs::loader::Message::Progress { n_done, .. } => {
                if n_done == LoadingProgress::Finished {
                    break;
                }
            }
            vfs::loader::Message::Loaded { files } => {
                let _p = tracing::info_span!("load_cargo::load_crate_craph/LoadedChanged").entered();
                for (path, contents) in files {
                    vfs.set_file_contents(path.into(), contents);
                }
            }
            vfs::loader::Message::Changed { files: _ } => {
                tracing::error!(?task, "unhandled vfs task");
            }
        }
    }
    let changes = vfs.take_changes();
    for (_, file) in changes {
        if let vfs::Change::Create(v, _) /*| vfs::Change::Modify(v, _)*/ = file.change {
            if let Ok(text) = String::from_utf8(v) {
                analysis_change.change_file(file.file_id, Some(text))
            }
        }
    }
    let package_root_config = project_folders.package_root_config;
    let package_roots = package_root_config.partition_into_package_roots(&vfs);
    analysis_change.set_package_roots(package_roots);

    db.apply_change(analysis_change);

    let builtins_change = builtins_file::add_to_vfs(&mut vfs);
    db.apply_change(builtins_change);

    Ok((db, vfs))
}
