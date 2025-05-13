use base_db::change::FileChanges;
use base_db::package_root::PackageRoot;
use ide::{Analysis, AnalysisHost};
use lang::builtin_files::BUILTINS_FILE;
use regex::Regex;
use std::cell::Cell;
use std::collections::HashSet;
use vfs::file_set::FileSet;
use vfs::{FileId, VfsPath};

const BUILTINS_FILE_ID: FileId = FileId::from_raw(0);

pub fn from_single_file(text: String) -> (Analysis, FileId) {
    let mut test_package = TestPackage::new();

    let mut changes = FileChanges::new();

    let mut file_set = FileSet::default();
    let file_id = test_package.new_file_id();
    file_set.insert(file_id, VfsPath::new_virtual_path("/main.move".to_owned()));

    changes.set_package_roots(vec![PackageRoot::new_local(file_set)]);
    changes.change_file(file_id, Some(text));

    test_package.apply_changes(changes);

    (test_package.analysis(), file_id)
}

pub fn from_multiple_files(source: &str) -> TestPackage {
    let files_source = stdx::trim_indent(source);

    let file_sep = Regex::new(r#"^\s*//- (\S+)\s*$"#).unwrap();

    let mut files: Vec<(String, String)> = vec![];
    let mut file_contents = vec![];
    let mut current_file_name: Option<String> = None;
    for line in files_source.lines() {
        let re = file_sep.captures(line);
        if let Some(re) = re {
            if current_file_name.is_some() {
                files.push((current_file_name.unwrap().clone(), file_contents.join("\n")));
                file_contents = vec![];
            }
            current_file_name = re.get(0).map(|it| it.as_str().to_string());
            continue;
        }
        if current_file_name.is_some() {
            file_contents.push(line);
        }
    }
    if current_file_name.is_some() {
        files.push((current_file_name.unwrap().clone(), file_contents.join("\n")));
        file_contents = vec![];
    }

    let mut test_package = TestPackage::new();

    let mut file_set = FileSet::default();
    let mut changes = FileChanges::new();
    for (file_name, file_contents) in files {
        let file_id = test_package.new_file_id();
        file_set.insert(file_id, VfsPath::new_virtual_path(file_name));
        changes.change_file(file_id, Some(file_contents));
    }

    let package_root = PackageRoot::new_local(file_set);
    changes.set_package_roots(vec![package_root]);

    test_package.apply_changes(changes);

    test_package
}

pub struct TestPackage {
    pub(crate) analysis_host: AnalysisHost,
    pub(crate) files: HashSet<FileId>,
    next_file_id: Cell<u32>,
}

impl TestPackage {
    pub fn new() -> TestPackage {
        let mut changes = FileChanges::new();
        changes.add_builtins_file(BUILTINS_FILE_ID, BUILTINS_FILE.to_string());

        let mut host = AnalysisHost::new();
        host.apply_change(changes);

        let mut files = HashSet::new();
        files.insert(BUILTINS_FILE_ID);

        TestPackage {
            analysis_host: host,
            files,
            next_file_id: Cell::new(1),
        }
    }

    pub fn file_with_caret(&self, caret: &str) -> (FileId, String) {
        for file_id in &self.files {
            let file_text = self.file_text(*file_id);
            if file_text.contains(caret) {
                return (*file_id, file_text);
            }
        }
        panic!("file with {caret} is missing");
    }

    pub fn analysis(&self) -> Analysis {
        self.analysis_host.analysis()
    }

    pub fn file_text(&self, file_id: FileId) -> String {
        self.analysis().file_text(file_id).unwrap().to_string()
    }

    pub(crate) fn apply_changes(&mut self, changes: FileChanges) {
        for (file_id, _) in &changes.files_changed {
            self.files.insert(file_id.to_owned());
        }
        self.analysis_host.apply_change(changes);
    }

    pub(crate) fn new_file_id(&self) -> FileId {
        let new_id = self.next_file_id.get();
        self.next_file_id.set(new_id + 1);
        FileId::from_raw(new_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_multiple_files() {
        // language=Move
        let test_package = from_multiple_files(
            r#"
//- /call.move
module 0x1::call {
    fun call() {}
}
//- /main.move
module 0x1::m {
    fun main() { /*caret*/ }
}
        "#,
        );
        assert_eq!(test_package.files.len(), 3);

        let (_, file_with_caret) = test_package.file_with_caret("/*caret*/").unwrap();
        assert_eq!(
            file_with_caret,
            // language=Move
            stdx::trim_indent(
                r#"
module 0x1::m {
    fun main() { /*caret*/ }
}
        "#
            )
        )
    }
}
