// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use base_db::SourceDatabase;
use base_db::change::FileChanges;
use base_db::package_root::{PackageKind, PackageRoot};
use ide_db::RootDatabase;
use std::panic::{AssertUnwindSafe, catch_unwind};
use vfs::file_set::FileSet;
use vfs::{FileId, VfsPath};

fn package_root(file_id: FileId, path: &str) -> PackageRoot {
    let mut file_set = FileSet::default();
    file_set.insert(file_id, VfsPath::new_virtual_path(path.to_owned()));
    PackageRoot::new(file_set, PackageKind::Local, None)
}

fn package_id_indexes(db: &RootDatabase) -> Vec<u32> {
    db.all_package_ids()
        .data(db)
        .iter()
        .map(|package_id| package_id.idx(db))
        .collect()
}

fn package_root_file_ids(db: &RootDatabase) -> Vec<Vec<FileId>> {
    db.all_package_ids()
        .data(db)
        .iter()
        .map(|package_id| {
            db.package_root(*package_id)
                .data(db)
                .file_ids()
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn adding_package_roots_updates_all_package_ids() {
    let mut db = RootDatabase::new();

    let mut changes = FileChanges::new();
    changes.set_package_roots(vec![package_root(FileId::from_raw(1), "/p1/main.move")]);
    db.apply_change(changes);

    assert_eq!(package_id_indexes(&db), vec![0]);
    assert_eq!(package_root_file_ids(&db), vec![vec![FileId::from_raw(1)]]);

    let mut changes = FileChanges::new();
    changes.set_package_roots(vec![
        package_root(FileId::from_raw(1), "/p1/main.move"),
        package_root(FileId::from_raw(2), "/p2/main.move"),
    ]);
    db.apply_change(changes);

    assert_eq!(package_id_indexes(&db), vec![0, 1]);
    assert_eq!(
        package_root_file_ids(&db),
        vec![vec![FileId::from_raw(1)], vec![FileId::from_raw(2)]]
    );
}

#[test]
fn replacing_package_roots_removes_stale_package_ids() {
    let mut db = RootDatabase::new();

    let mut changes = FileChanges::new();
    changes.set_package_roots(vec![
        package_root(FileId::from_raw(1), "/p1/main.move"),
        package_root(FileId::from_raw(2), "/p2/main.move"),
    ]);
    db.apply_change(changes);

    assert_eq!(package_id_indexes(&db), vec![0, 1]);
    assert_eq!(
        package_root_file_ids(&db),
        vec![vec![FileId::from_raw(1)], vec![FileId::from_raw(2)]]
    );

    let mut changes = FileChanges::new();
    changes.set_package_roots(vec![package_root(FileId::from_raw(1), "/p1/main.move")]);
    db.apply_change(changes);

    assert_eq!(package_id_indexes(&db), vec![0]);
    assert_eq!(package_root_file_ids(&db), vec![vec![FileId::from_raw(1)]]);
}

#[test]
fn replacing_package_roots_removes_stale_file_package_ids() {
    let mut db = RootDatabase::new();

    let mut changes = FileChanges::new();
    changes.set_package_roots(vec![
        package_root(FileId::from_raw(1), "/p1/main.move"),
        package_root(FileId::from_raw(2), "/p2/main.move"),
    ]);
    db.apply_change(changes);

    assert_eq!(db.file_package_id(FileId::from_raw(1)).idx(&db), 0);
    assert_eq!(db.file_package_id(FileId::from_raw(2)).idx(&db), 1);

    let mut changes = FileChanges::new();
    changes.set_package_roots(vec![package_root(FileId::from_raw(1), "/p1/main.move")]);
    db.apply_change(changes);

    assert_eq!(db.file_package_id(FileId::from_raw(1)).idx(&db), 0);
    let stale_file_lookup = catch_unwind(AssertUnwindSafe(|| db.file_package_id(FileId::from_raw(2))));
    assert!(stale_file_lookup.is_err());
}
