use anyhow::{Context, bail};
use paths::AbsPathBuf;
use std::fs;
use std::path::Display;
use std::process::Command;

pub mod aptos_package;
pub mod dep_graph;
pub mod manifest_path;
pub mod move_toml;
pub mod project_folders;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct DiscoveredManifest {
    pub move_toml_file: AbsPathBuf,
    pub resolve_deps: bool,
}

impl DiscoveredManifest {
    pub fn discover_all(ws_roots: &[AbsPathBuf]) -> Vec<DiscoveredManifest> {
        let mut all_manifests = vec![];
        for ws_root in ws_roots {
            let manifests = walk_and_discover_manifests(ws_root);
            all_manifests.extend(manifests);
        }
        all_manifests.sort();
        all_manifests.dedup();
        all_manifests
    }

    pub fn display_root(&self) -> String {
        self.move_toml_file
            .parent()
            .map(|it| it.to_string())
            .expect("Move.toml file should have a parent")
    }
}

fn walk_and_discover_manifests(ws_root: &AbsPathBuf) -> Vec<DiscoveredManifest> {
    let candidate = ws_root.join("aptos-move").join("framework");
    let aptos_core_dirs = match fs::exists(&candidate) {
        Ok(true) => {
            let aptos_core_dirs = vec![
                ws_root.join("aptos-move").join("framework"),
                ws_root.join("aptos-move").join("move-examples"),
                ws_root
                    .join("testsuite")
                    .join("module-publish")
                    .join("src")
                    .join("packages"),
            ];
            let dirs_to_resolve = aptos_core_dirs
                .clone()
                .into_iter()
                .map(|it| it.to_string())
                .collect::<Vec<_>>();
            tracing::error!(
                "aptos-core repository detected, dependency resolution is restricted to {:#?}",
                dirs_to_resolve,
            );
            Some(aptos_core_dirs)
        }
        _ => None,
    };

    let mut manifests = vec![];
    for entry in walkdir::WalkDir::new(ws_root)
        .into_iter()
        .filter_map(|it| it.ok())
    {
        let path = entry.path();
        let resolve_deps = aptos_core_dirs
            .clone()
            .is_none_or(|dirs| dirs.iter().any(|dir| path.starts_with(dir)));
        let mfile_path = path.join("Move.toml");
        if mfile_path.exists() {
            let m = DiscoveredManifest {
                move_toml_file: AbsPathBuf::assert_utf8(mfile_path),
                resolve_deps,
            };
            manifests.push(m);
        }
    }
    manifests
}

fn utf8_stdout(cmd: &mut Command) -> anyhow::Result<String> {
    let output = cmd.output().with_context(|| format!("{cmd:?} failed"))?;
    if !output.status.success() {
        match String::from_utf8(output.stderr) {
            Ok(stderr) if !stderr.is_empty() => {
                bail!("{:?} failed, {}\nstderr:\n{}", cmd, output.status, stderr)
            }
            _ => bail!("{:?} failed, {}", cmd, output.status),
        }
    }
    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout.trim().to_owned())
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum InvocationStrategy {
    Once,
    #[default]
    PerWorkspace,
}
