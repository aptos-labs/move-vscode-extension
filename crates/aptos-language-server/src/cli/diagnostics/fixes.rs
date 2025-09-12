// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use base_db::change::FileChanges;
use ide_db::RootDatabase;
use ide_db::assists::Assist;
use ide_diagnostics::diagnostic::Diagnostic;
use std::fs;
use syntax::TextRange;
use vfs::{FileId, Vfs};

#[derive(Debug, PartialEq, Eq, Clone)]
pub(super) enum FixCodes {
    None,
    Codes(Vec<String>),
    All,
}

impl FixCodes {
    pub(super) fn from_cli(apply_fixes: Option<&Vec<String>>) -> Self {
        match apply_fixes {
            None => FixCodes::None,
            Some(codes) if codes.contains(&"all".to_string()) => FixCodes::All,
            Some(codes) => FixCodes::Codes(codes.clone()),
        }
    }

    pub(super) fn has_codes_to_apply(&self) -> bool {
        matches!(self, FixCodes::Codes(_) | FixCodes::All)
    }
}

pub(super) fn apply_fix(fix: &Assist, before: &str) -> (String, Vec<TextRange>) {
    let source_change = fix.source_change.as_ref().unwrap();
    let mut after = before.to_string();
    let mut new_text_ranges = vec![];
    for text_edit in source_change.source_file_edits.values() {
        new_text_ranges.extend(text_edit.iter().map(|it| it.new_range()));
        text_edit.apply(&mut after);
    }
    (after, new_text_ranges)
}

pub(super) fn find_diagnostic_with_fix(
    diagnostics: Vec<Diagnostic>,
    fix_codes: &FixCodes,
) -> Option<(Diagnostic, Assist)> {
    for diagnostic in diagnostics {
        let fixes = diagnostic.fixes.clone().unwrap_or_default();
        for fix in fixes {
            match fix_codes {
                FixCodes::All => return Some((diagnostic, fix)),
                FixCodes::Codes(allowed_codes) => {
                    if allowed_codes.contains(&fix.id.0.to_string()) {
                        return Some((diagnostic, fix));
                    }
                }
                FixCodes::None => unreachable!(),
            }
        }
    }
    None
}

pub(super) fn write_file_text(
    vfs: &mut Vfs,
    db: &mut RootDatabase,
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
