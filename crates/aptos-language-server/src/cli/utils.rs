use base_db::SourceDatabase;
use base_db::change::FileChanges;
use base_db::package_root::PackageRoot;
use camino::Utf8PathBuf;
use ide_db::RootDatabase;
use ide_db::assists::Assist;
use paths::AbsPathBuf;
use project_model::DiscoveredManifest;
use project_model::aptos_package::load_from_fs;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use syntax::TextRange;
use vfs::{FileId, Vfs};

pub(crate) fn init_db(manifests: Vec<DiscoveredManifest>) -> (RootDatabase, vfs::Vfs) {
    let aptos_packages = load_from_fs::load_aptos_packages(manifests).valid_packages();
    ide_db::load::load_db(&aptos_packages).unwrap()
}

pub(crate) fn all_package_roots(db: &RootDatabase) -> Vec<Arc<PackageRoot>> {
    db.all_package_ids()
        .data(db)
        .into_iter()
        .map(|it| db.package_root(it).data(db))
        .filter(|it| !it.is_builtin())
        .collect::<Vec<_>>()
}

pub(crate) fn ws_package_roots(
    db: &RootDatabase,
    vfs: &Vfs,
    ws_root: AbsPathBuf,
) -> Vec<Arc<PackageRoot>> {
    let canonical_ws_root = AbsPathBuf::assert_utf8(fs::canonicalize(ws_root).unwrap());
    let local_package_roots = all_package_roots(&db)
        .into_iter()
        .filter(|package_root| {
            !package_root.is_library()
                && package_root
                    .root_dir(&vfs)
                    .is_some_and(|it| it.starts_with(&canonical_ws_root))
        })
        .collect::<Vec<_>>();
    local_package_roots
}

pub(crate) fn all_roots_file_ids(db: &RootDatabase) -> Vec<FileId> {
    all_package_roots(db)
        .iter()
        .flat_map(|it| it.file_ids())
        .collect()
}

pub(crate) fn find_target_file_id(
    db: &RootDatabase,
    vfs: &Vfs,
    target_path: AbsPathBuf,
) -> Option<FileId> {
    all_roots_file_ids(&db)
        .into_iter()
        .find(|file_id| vfs.file_path(*file_id).as_path().unwrap().to_path_buf() == target_path)
}

pub(crate) fn apply_assist(assist: &Assist, before: &str) -> (String, Vec<TextRange>) {
    let source_change = assist.source_change.as_ref().unwrap();
    let mut after = before.to_string();
    let mut new_text_ranges = vec![];
    for text_edit in source_change.source_file_edits.values() {
        new_text_ranges.extend(text_edit.iter().map(|it| it.new_range()));
        text_edit.apply(&mut after);
    }
    (after, new_text_ranges)
}

pub(crate) fn write_file_text(
    db: &mut RootDatabase,
    vfs: &mut Vfs,
    file_id: FileId,
    new_file_text: &String,
) {
    let mut change = FileChanges::new();
    change.change_file(file_id, Some(new_file_text.clone()));
    db.apply_change(change);

    let file_path = vfs.file_path(file_id).to_owned();
    let abs_file_path = file_path.as_path().unwrap().to_path_buf();

    vfs.set_file_contents(file_path, Some(new_file_text.clone().into_bytes()));
    fs::write(&abs_file_path, new_file_text.clone()).expect("cannot write file");
}

pub(super) struct CmdPath {
    path: Utf8PathBuf,
}

pub(super) enum CmdPathKind {
    Workspace(AbsPathBuf),
    MoveFile(AbsPathBuf),
    Other,
}

impl CmdPath {
    pub fn new(path: &PathBuf) -> anyhow::Result<Self> {
        let provided_path = Utf8PathBuf::from_path_buf(std::env::current_dir()?.join(path)).unwrap();
        Ok(CmdPath { path: provided_path })
    }

    pub fn kind(&self) -> CmdPathKind {
        let abs_path = AbsPathBuf::assert(self.path.clone());
        if self.path.is_file() && self.path.extension() == Some("move") {
            CmdPathKind::MoveFile(abs_path)
        } else {
            if self.path.is_dir() {
                CmdPathKind::Workspace(abs_path)
            } else {
                CmdPathKind::Other
            }
        }
    }
}
