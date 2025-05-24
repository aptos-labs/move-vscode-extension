use crate::aptos_package::{AptosPackage, VfsLoader};
use crate::project_folders::PackageRootConfig;
use base_db::change::{FileChanges, MoveTomlFileId, PackageGraph};
use paths::AbsPath;
use vfs::Vfs;

pub fn collect_initial(packages: &[AptosPackage], vfs: &mut Vfs) -> Option<PackageGraph> {
    let mut load = |path: &AbsPath| {
        let contents = std::fs::read(path).ok();
        let path = vfs::VfsPath::from(path.to_path_buf());
        vfs.set_file_contents(path.clone(), contents);
        vfs.file_id(&path)
            .and_then(|(file_id, excluded)| (excluded == vfs::FileExcluded::No).then_some(file_id))
    };
    collect(packages, &mut load)
}

pub fn collect(aptos_packages: &[AptosPackage], load: VfsLoader<'_>) -> Option<PackageGraph> {
    let _p = tracing::info_span!("dep_graph::collect").entered();

    let mut global_dep_graph = PackageGraph::default();

    for package in aptos_packages.iter() {
        let (package_file_id, dep_ids) = package.dep_graph_entry(load)?;
        global_dep_graph.insert(package_file_id, dep_ids);
    }

    Some(global_dep_graph)
}

impl AptosPackage {
    fn dep_graph_entry(&self, load: VfsLoader<'_>) -> Option<(MoveTomlFileId, Vec<MoveTomlFileId>)> {
        let package_file_id = load_package_file_id(self.content_root(), load)?;

        let mut dep_ids = vec![];
        for (dep_root, _) in self.dep_roots() {
            let dep_file_id = load_package_file_id(dep_root, load)?;
            dep_ids.push(dep_file_id);
        }

        Some((package_file_id, dep_ids))
    }
}

pub fn log_dependencies(dep_graph: &PackageGraph, vfs: &Vfs) {
    for (package_file_id, dep_ids) in dep_graph {
        let main_package_name = vfs
            .file_path(*package_file_id)
            .as_path()
            .and_then(|it| it.file_name());
        let dep_names = dep_ids
            .iter()
            .map(|it| vfs.file_path(*it).as_path().and_then(|p| p.file_name()));
        tracing::debug!(?main_package_name, ?dep_names);
    }
}

fn load_package_file_id(dep_root: &AbsPath, load_from_vfs: VfsLoader<'_>) -> Option<MoveTomlFileId> {
    let move_toml_file = dep_root.join("Move.toml");
    match load_from_vfs(&move_toml_file) {
        Some(file_id) => Some(file_id),
        None => {
            tracing::info!(?move_toml_file, "cannot load from filesystem");
            None
        }
    }
}
