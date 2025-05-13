use crate::aptos_package::{AptosPackage, FileLoader};
use crate::project_folders::PackageRootConfig;
use base_db::change::{DepGraph, FileChanges, ManifestFileId};
use paths::AbsPath;
use vfs::Vfs;

pub fn reload_graph(
    vfs: &Vfs,
    aptos_packages: &[AptosPackage],
    package_root_config: &PackageRootConfig,
) -> Option<FileChanges> {
    let dep_graph = collect(vfs, aptos_packages)?;

    let mut change = FileChanges::new();
    {
        let package_roots = package_root_config.partition_into_package_roots(vfs);
        change.set_package_roots(package_roots);
        // depends on roots being available
        change.set_package_graph(dep_graph);
    }
    Some(change)
}

fn collect(vfs: &Vfs, aptos_packages: &[AptosPackage]) -> Option<DepGraph> {
    let _p = tracing::info_span!("dep_graph::collect").entered();

    let mut global_dep_graph = DepGraph::default();

    let mut load = |path: &AbsPath| {
        vfs.file_id(&vfs::VfsPath::from(path.to_path_buf()))
            .map(|it| it.0)
    };

    for package in aptos_packages.iter() {
        let dep_graph = package.to_dep_graph(&mut load)?;
        global_dep_graph.extend(dep_graph);
    }

    Some(global_dep_graph)
}

impl AptosPackage {
    pub fn to_dep_graph(&self, load: FileLoader<'_>) -> Option<DepGraph> {
        tracing::info!("reloading package at {}", self.content_root());

        let mut package_graph = DepGraph::default();
        for pkg in self.package_and_deps() {
            let package_file_id = pkg.load_manifest_file_id(load)?;
            let mut dep_ids = vec![];
            self.collect_dep_ids(&mut dep_ids, pkg, load);
            dep_ids.sort();
            dep_ids.dedup();

            package_graph.insert(package_file_id, dep_ids);
        }

        Some(package_graph)
    }

    fn collect_dep_ids(
        &self,
        dep_ids: &mut Vec<ManifestFileId>,
        package_ref: &AptosPackage,
        load: FileLoader<'_>,
    ) {
        for dep_package in package_ref.deps() {
            if let Some(dep_file_id) = dep_package.load_manifest_file_id(load) {
                dep_ids.push(dep_file_id);
                self.collect_dep_ids(dep_ids, dep_package, load);
            }
        }
    }
}
