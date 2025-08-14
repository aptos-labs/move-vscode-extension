// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;
use std::{env, fs};

const COPYRIGHT_NOTICE_ORIGINAL: &str = r#"
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

"#;
const COPYRIGHT_NOTICE_WITH_RUST_ANALYZER: &str = r#"
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

"#;

const ORIGINAL_FILE_PATTERNS: &[&str] = &[
    "ide-tests/",
    "syntax/src/ast/node_ext",
    "syntax/src/syntax_editor/node_ext",
    "ide-diagnostics/src/handlers",
    "ide-completion/src/render",
    "lang/",
    "aptos-language-server/src/cli/",
];

pub fn enforce() {
    let ws_root = env::current_dir().unwrap();
    let crates_dir = ws_root.join("crates");
    for entry in walkdir::WalkDir::new(&crates_dir)
        .into_iter()
        .filter_map(|it| it.ok())
    {
        if entry.path().extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        enforce_copyright_in_file(crates_dir.as_path(), entry.path());
    }
}

fn enforce_copyright_in_file(root: &Path, file_path: &Path) -> Option<()> {
    let file_text = fs::read_to_string(file_path).ok()?;
    // no copyright at all
    if !starts_with_copyright(&file_text, COPYRIGHT_NOTICE_ORIGINAL) {
        println!("updating copyright for file `{}`", file_path.display());
        let notice = if is_original(root, file_path) {
            COPYRIGHT_NOTICE_ORIGINAL.trim_start()
        } else {
            COPYRIGHT_NOTICE_WITH_RUST_ANALYZER.trim_start()
        };
        enforce_notice(file_path, notice);
    }
    Some(())
}

fn starts_with_copyright(text: impl Into<String>, notice: &str) -> bool {
    text.into().starts_with(notice.trim_start())
}

fn enforce_notice(file_path: &Path, notice: &str) -> Option<()> {
    let file_text = fs::read_to_string(file_path).ok()?;
    fs::write(file_path, format!("{notice}{file_text}")).ok()?;
    Some(())
}

fn is_original(root: &Path, file_path: &Path) -> bool {
    file_path
        .strip_prefix(root)
        .ok()
        .map(|relpath| ORIGINAL_FILE_PATTERNS.iter().any(|it| relpath.starts_with(it)))
        .unwrap_or(false)
}
