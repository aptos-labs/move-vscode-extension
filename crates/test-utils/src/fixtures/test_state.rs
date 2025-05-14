use crate::fixtures::parse_files_from_source;
use base_db::change::FileChanges;
use ide::{Analysis, AnalysisHost};
use lang::builtins_file;
use project_model::aptos_package::AptosPackage;
use project_model::aptos_package::load_from_fs::load_aptos_packages;
use project_model::project_folders::ProjectFolders;
use project_model::{DiscoveredManifest, dep_graph};
use std::fs;
use std::path::PathBuf;
use stdx::itertools::Itertools;
use vfs::{AbsPathBuf, FileId, Vfs, VfsPath};

pub fn from_multiple_files_on_tmpfs(test_packages: Vec<TestPackageFiles>) -> TestState {
    let tmp = tempdir::TempDir::new("aptos_analyzer_tests").unwrap();

    let mut vfs = Vfs::default();

    let ws_root = tmp.path().join("ws_root");
    fs::create_dir(&ws_root).unwrap();

    let mut analysis_host = AnalysisHost::default();
    analysis_host.apply_change(builtins_file::add_to_vfs(&mut vfs));

    for test_package in test_packages {
        let mut file_changes = FileChanges::new();

        let package_root = ws_root.join(test_package.root_dir);
        fs::create_dir(&package_root).unwrap();

        let move_toml_file = package_root.join("Move.toml");
        let move_toml_contents = test_package.move_toml;
        create_new_test_file(&mut vfs, &mut file_changes, &move_toml_file, &move_toml_contents);

        let sources_dir = package_root.join("sources");
        fs::create_dir(&sources_dir).unwrap();

        let files = parse_files_from_source(&test_package.source_files);
        for (file_name, file_text) in files {
            let fpath = sources_dir.join(file_name.trim_start_matches("/"));
            create_new_test_file(&mut vfs, &mut file_changes, &fpath, &file_text);
        }
        analysis_host.apply_change(file_changes);
    }

    let discovered_manifests = DiscoveredManifest::discover_all(&[AbsPathBuf::assert_utf8(ws_root)]);
    let packages = load_aptos_packages(discovered_manifests)
        .into_iter()
        .filter_map(|it| it.ok())
        .collect::<Vec<_>>();
    let folders = ProjectFolders::new(&packages);
    let dep_graph_change =
        dep_graph::reload_graph(&vfs, &packages, &folders.package_root_config).unwrap();
    analysis_host.apply_change(dep_graph_change);

    TestState { packages, vfs, analysis_host }
}

pub struct TestPackageFiles {
    move_toml: String,
    root_dir: String,
    source_files: String,
}

impl TestPackageFiles {
    pub fn new(root_dir: &str, move_toml: &str, source_files: &str) -> Self {
        TestPackageFiles {
            root_dir: root_dir.to_string(),
            move_toml: stdx::trim_indent(move_toml),
            source_files: source_files.to_string(),
        }
    }

    pub fn named(name: &str, source_files: &str) -> Self {
        // language=TOML
        TestPackageFiles {
            root_dir: name.to_string(),
            move_toml: stdx::trim_indent(&format!(
                r#"
[package]
name = "{name}"
version = "0.1.0"
        "#
            )),
            source_files: source_files.to_string(),
        }
    }
}

pub struct TestState {
    vfs: Vfs,
    analysis_host: AnalysisHost,
    packages: Vec<AptosPackage>,
}

impl TestState {
    pub fn analysis(&self) -> Analysis {
        self.analysis_host.analysis()
    }

    pub fn file_with_caret(&self, caret: &str) -> (FileId, String) {
        let analysis = self.analysis_host.analysis();
        for (file_id, _) in self.vfs.iter() {
            let file_text = analysis.file_text(file_id).unwrap().to_string();
            if file_text.contains(caret) {
                return (file_id, file_text);
            }
        }
        panic!("file with {caret} is missing");
    }
}

fn create_new_test_file(vfs: &mut Vfs, change: &mut FileChanges, fpath: &PathBuf, contents: &str) {
    fs::write(&fpath, contents).unwrap();

    let vfs_path = VfsPath::new_real_path(fpath.to_str().unwrap().to_string());
    vfs.set_file_contents(vfs_path.clone(), Some(contents.bytes().collect()));

    let (file_id, _) = vfs.file_id(&vfs_path).unwrap();
    change.files_changed.push((file_id, Some(contents.to_string())));
}
