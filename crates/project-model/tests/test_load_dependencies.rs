use paths::AbsPathBuf;
use project_model::DiscoveredManifest;
use project_model::aptos_package::AptosPackage;
use project_model::manifest_path::ManifestPath;
use std::env::current_dir;

#[test]
fn test_circular_dependencies() {
    let ws_dir = current_dir()
        .unwrap()
        .join("tests")
        .join("circular_dependencies")
        .join("Move.toml");
    let move_toml_path = AbsPathBuf::assert_utf8(ws_dir);
    let manifest = ManifestPath::new(move_toml_path);
    AptosPackage::load(&manifest, true).unwrap();
}

#[test]
fn test_if_inside_aptos_core_only_load_deps_from_aptos_move() {
    let root = current_dir().unwrap().join("tests").join("aptos-core");
    let ws_roots = vec![AbsPathBuf::assert_utf8(root)];
    let discovered_manifests = DiscoveredManifest::discover_all(&ws_roots);
    assert_eq!(discovered_manifests.len(), 2);
    assert_eq!(
        discovered_manifests
            .iter()
            .map(|it| it.resolve_deps)
            .collect::<Vec<_>>(),
        vec![true, false]
    );
}
