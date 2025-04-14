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

    pub fn discover_single(path: &AbsPath) -> anyhow::Result<ManifestPath> {
        let mut candidates = ManifestPath::discover(path)?;
        let res = match candidates.pop() {
            None => bail!("no projects"),
            Some(it) => it,
        };

        if !candidates.is_empty() {
            bail!("more than one project");
        }
        Ok(res)
    }

    pub fn discover(path: &AbsPath) -> io::Result<Vec<ManifestPath>> {
        return match find_in_parent_dirs(path, "Move.toml") {
            Some(it) => Ok(vec![it]),
            None => Ok(find_move_toml_in_child_dir(read_dir(path)?)),
        };

        fn find_in_parent_dirs(path: &AbsPath, target_file_name: &str) -> Option<ManifestPath> {
            if path.file_name().unwrap_or_default() == target_file_name {
                if let Ok(manifest) = ManifestPath::try_from(path.to_path_buf()) {
                    return Some(manifest);
                }
            }

            let mut curr = Some(path);

            while let Some(path) = curr {
                let candidate = path.join(target_file_name);
                if fs::exists(&candidate).is_ok() {
                    if let Ok(manifest) = ManifestPath::try_from(candidate) {
                        return Some(manifest);
                    }
                }
                curr = path.parent();
            }

            None
        }

        fn find_move_toml_in_child_dir(entities: ReadDir) -> Vec<ManifestPath> {
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
    }

    pub fn discover_all(paths: &[AbsPathBuf]) -> Vec<ManifestPath> {
        let mut res = paths
            .iter()
            .filter_map(|it| ManifestPath::discover(it.as_ref()).ok())
            .flatten()
            .collect::<FxHashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        res.sort();
        res
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
