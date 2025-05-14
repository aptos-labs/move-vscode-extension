use paths::AbsPathBuf;
use project_model::DiscoveredManifest;
use project_model::aptos_package::load_from_fs;
use project_model::aptos_package::load_from_fs::load_aptos_packages;
use std::env::current_dir;

#[test]
fn test_circular_dependencies() {
    let ws_dir = current_dir()
        .unwrap()
        .join("tests")
        .join("circular_dependencies")
        .join("Move.toml");
    let manifest = DiscoveredManifest {
        move_toml_file: AbsPathBuf::assert_utf8(ws_dir),
        resolve_deps: true,
    };
    load_from_fs::load_aptos_packages(vec![manifest]);
}

#[test]
fn test_if_inside_aptos_core_only_load_deps_from_aptos_move() {
    let root = current_dir().unwrap().join("tests").join("aptos-core");
    let ws_roots = vec![AbsPathBuf::assert_utf8(root)];
    let discovered_manifests = DiscoveredManifest::discover_all(&ws_roots);

    let packages = load_aptos_packages(discovered_manifests)
        .into_iter()
        .map(|it| it.unwrap())
        .collect::<Vec<_>>();
    assert_eq!(packages.len(), 3);

    let other_movestdlib_package = packages
        .iter()
        .find(|it| it.content_root().to_string().contains("other-move"))
        .unwrap();
    assert_eq!(other_movestdlib_package.dep_roots().len(), 0);

    let aptos_stdlib = packages
        .iter()
        .find(|it| it.content_root().to_string().contains("aptos-stdlib"))
        .unwrap();
    assert_eq!(aptos_stdlib.dep_roots().len(), 1);
}
