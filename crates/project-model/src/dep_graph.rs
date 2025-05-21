use crate::aptos_package::{AptosPackage, VfsLoader};
use crate::project_folders::PackageRootConfig;
use base_db::change::{FileChanges, PackageFileId, PackageGraph};
use paths::AbsPath;

pub fn reload_graph(
    vfs: &vfs::Vfs,
    aptos_packages: &[AptosPackage],
    package_root_config: &PackageRootConfig,
) -> Option<FileChanges> {
    let mut load = |path: &AbsPath| {
        tracing::debug!(?path, "load from vfs");
        vfs.file_id(&vfs::VfsPath::from(path.to_path_buf()))
            .map(|it| it.0)
    };
    let package_graph = collect(aptos_packages, &mut load)?;

    let mut change = FileChanges::new();
    {
        let package_roots = package_root_config.partition_into_package_roots(vfs);
        change.set_package_roots(package_roots);
        // depends on roots being available
        change.set_package_graph(package_graph);
    }
    Some(change)
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
    pub fn dep_graph_entry(&self, load: VfsLoader<'_>) -> Option<(PackageFileId, Vec<PackageFileId>)> {
        tracing::info!("reloading package at {}", self.content_root());

        let package_file_id = load_package_file_id(self.content_root(), load)?;

        let mut dep_ids = vec![];
        for (dep_root, _) in self.dep_roots() {
            let dep_file_id = load_package_file_id(dep_root, load)?;
            dep_ids.push(dep_file_id);
        }

        Some((package_file_id, dep_ids))
    }
}

fn load_package_file_id(dep_root: &AbsPath, load_from_vfs: VfsLoader<'_>) -> Option<PackageFileId> {
    let move_toml_file = dep_root.join("Move.toml");
    match load_from_vfs(&move_toml_file) {
        Some(file_id) => Some(file_id),
        None => {
            tracing::info!(?move_toml_file, "cannot load from filesystem");
            None
        }
    }
}
