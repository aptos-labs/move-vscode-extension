use crate::manifest_path::ManifestPath;
use anyhow::{Context, bail};
use paths::{AbsPath, AbsPathBuf, Utf8PathBuf};
use rustc_hash::FxHashSet;
use std::fs::{ReadDir, read_dir};
use std::process::Command;
use std::{fs, io};

pub mod aptos_package;
pub mod aptos_workspace;
pub mod manifest_path;
pub mod move_toml;

pub use aptos_workspace::AptosWorkspace;

impl ManifestPath {
    pub fn from_manifest_file(file: AbsPathBuf) -> anyhow::Result<ManifestPath> {
        if file.file_name().unwrap_or_default() == "Move.toml" {
            return Ok(ManifestPath { file });
        }
        bail!("project root must point to a Cargo.toml file: {file}");
    }

    // pub fn discover_single(path: &AbsPath) -> anyhow::Result<ManifestPath> {
    //     let mut candidates = ManifestPath::discover(path)?;
    //     let res = match candidates.pop() {
    //         None => bail!("no projects"),
    //         Some(it) => it,
    //     };
    //
    //     if !candidates.is_empty() {
    //         bail!("more than one project");
    //     }
    //     Ok(res)
    // }

    pub fn discover(ws_root: &AbsPath) -> io::Result<Vec<ManifestPath>> {
        let mut manifests = vec![];
        let root_manifest = ws_root.join("Move.toml");
        if fs::exists(&root_manifest)? {
            manifests.push(ManifestPath { file: root_manifest });
        }
        manifests.extend(Self::find_manifests_in_child_directories(read_dir(ws_root)?));
        Ok(manifests)
    }

    fn find_manifests_in_child_directories(entities: ReadDir) -> Vec<ManifestPath> {
        // Only one level down to avoid cycles the easy way and stop a runaway scan with large projects
        entities
            .filter_map(Result::ok)
            .map(|it| it.path().join("Move.toml"))
            .filter(|it| it.exists())
            .map(Utf8PathBuf::from_path_buf)
            .filter_map(Result::ok)
            .map(AbsPathBuf::try_from)
            .filter_map(Result::ok)
            .filter_map(|it| it.try_into().ok())
            .collect()
    }

    pub fn discover_all(ws_roots: &[AbsPathBuf]) -> Vec<ManifestPath> {
        let mut manifests = ws_roots
            .iter()
            .filter_map(|ws_root| ManifestPath::discover(ws_root.as_ref()).ok())
            .flatten()
            .collect::<FxHashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        manifests.sort();
        manifests
    }
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
