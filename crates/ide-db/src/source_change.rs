//! This modules defines type to represent changes to the source code, that flow
//! from the server to the client.
//!
//! It can be viewed as a dual for `Change`.

use crate::assists::Command;
use crate::syntax_helpers::tree_diff::tree_diff;
use crate::text_edit::{TextEdit, TextEditBuilder};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::{iter, mem};
use stdx::never;
use syntax::syntax_editor::SyntaxEditor;
use syntax::{AstNode, SyntaxNode, SyntaxNodePtr, TextRange, TextSize};
use vfs::{AnchoredPathBuf, FileId};

#[derive(Default, Debug, Clone)]
pub struct SourceChange {
    pub source_file_edits: HashMap<FileId, TextEdit>,
    pub file_system_edits: Vec<FileSystemEdit>,
}

impl SourceChange {
    /// Creates a new SourceChange with the given label
    /// from the edits.
    pub fn from_edits(
        source_file_edits: HashMap<FileId, TextEdit>,
        file_system_edits: Vec<FileSystemEdit>,
    ) -> Self {
        SourceChange {
            source_file_edits,
            file_system_edits,
        }
    }

    pub fn from_text_edit(file_id: impl Into<FileId>, edit: TextEdit) -> Self {
        SourceChange {
            source_file_edits: iter::once((file_id.into(), edit)).collect(),
            ..Default::default()
        }
    }

    /// Inserts a [`TextEdit`] for the given [`FileId`]. This properly handles merging existing
    /// edits for a file if some already exist.
    pub fn insert_source_edit(&mut self, file_id: impl Into<FileId>, edit: TextEdit) {
        match self.source_file_edits.entry(file_id.into()) {
            Entry::Occupied(mut entry) => {
                let value = entry.get_mut();
                never!(value.union(edit).is_err(), "overlapping edits for same file");
            }
            Entry::Vacant(entry) => {
                entry.insert(edit);
            }
        }
    }

    pub fn push_file_system_edit(&mut self, edit: FileSystemEdit) {
        self.file_system_edits.push(edit);
    }

    pub fn get_source_edit(&self, file_id: FileId) -> Option<&TextEdit> {
        self.source_file_edits.get(&file_id)
    }

    pub fn merge(mut self, other: SourceChange) -> SourceChange {
        self.extend(other.source_file_edits);
        self.extend(other.file_system_edits);
        self
    }
}

impl Extend<(FileId, TextEdit)> for SourceChange {
    fn extend<T: IntoIterator<Item = (FileId, TextEdit)>>(&mut self, iter: T) {
        iter.into_iter()
            .for_each(|(file_id, edit)| self.insert_source_edit(file_id, edit));
    }
}

impl Extend<FileSystemEdit> for SourceChange {
    fn extend<T: IntoIterator<Item = FileSystemEdit>>(&mut self, iter: T) {
        iter.into_iter().for_each(|edit| self.push_file_system_edit(edit));
    }
}

impl From<HashMap<FileId, TextEdit>> for SourceChange {
    fn from(source_file_edits: HashMap<FileId, TextEdit>) -> SourceChange {
        let source_file_edits = source_file_edits
            .into_iter()
            .map(|(file_id, edit)| (file_id, edit))
            .collect();
        SourceChange {
            source_file_edits,
            file_system_edits: Vec::new(),
        }
    }
}

impl FromIterator<(FileId, TextEdit)> for SourceChange {
    fn from_iter<T: IntoIterator<Item = (FileId, TextEdit)>>(iter: T) -> Self {
        let mut this = SourceChange::default();
        this.extend(iter);
        this
    }
}

pub struct SourceChangeBuilder {
    pub edit: TextEditBuilder,
    pub file_id: FileId,
    pub source_change: SourceChange,
    pub command: Option<Command>,

    /// Keeps track of all edits performed on each file
    pub file_editors: HashMap<FileId, SyntaxEditor>,

    /// Maps the original, immutable `SyntaxNode` to a `clone_for_update` twin.
    pub mutated_tree: Option<TreeMutator>,
}

pub struct TreeMutator {
    immutable: SyntaxNode,
    mutable_clone: SyntaxNode,
}

impl TreeMutator {
    pub fn new(immutable: &SyntaxNode) -> TreeMutator {
        let immutable = immutable.ancestors().last().unwrap();
        let mutable_clone = immutable.clone_for_update();
        TreeMutator { immutable, mutable_clone }
    }

    pub fn make_mut<N: AstNode>(&self, node: &N) -> N {
        N::cast(self.make_syntax_mut(node.syntax())).unwrap()
    }

    pub fn make_syntax_mut(&self, node: &SyntaxNode) -> SyntaxNode {
        let ptr = SyntaxNodePtr::new(node);
        ptr.to_node(&self.mutable_clone)
    }
}

impl SourceChangeBuilder {
    pub fn new(file_id: impl Into<FileId>) -> SourceChangeBuilder {
        SourceChangeBuilder {
            edit: TextEdit::builder(),
            file_id: file_id.into(),
            source_change: SourceChange::default(),
            command: None,
            file_editors: HashMap::default(),
            mutated_tree: None,
        }
    }

    pub fn edit_file(&mut self, file_id: impl Into<FileId>) {
        self.file_id = file_id.into();
    }

    pub fn make_editor(&self, node: &SyntaxNode) -> SyntaxEditor {
        SyntaxEditor::new(node.ancestors().last().unwrap_or_else(|| node.clone()))
    }

    pub fn add_file_edits(&mut self, file_id: impl Into<FileId>, edit: SyntaxEditor) {
        match self.file_editors.entry(file_id.into()) {
            Entry::Occupied(mut entry) => entry.get_mut().merge(edit),
            Entry::Vacant(entry) => {
                entry.insert(edit);
            }
        }
    }

    fn commit(&mut self) {
        // Apply syntax editor edits
        for (file_id, editor) in mem::take(&mut self.file_editors) {
            let edit_result = editor.finish();

            let mut edit = TextEdit::builder();
            tree_diff(edit_result.old_root(), edit_result.new_root()).into_text_edit(&mut edit);
            let edit = edit.finish();

            if !edit.is_empty() {
                self.source_change.insert_source_edit(file_id, edit);
            }
        }

        if let Some(tm) = self.mutated_tree.take() {
            tree_diff(&tm.immutable, &tm.mutable_clone).into_text_edit(&mut self.edit);
        }

        let edit = mem::take(&mut self.edit).finish();
        if !edit.is_empty() {
            self.source_change.insert_source_edit(self.file_id, edit);
        }
    }

    pub fn make_mut<N: AstNode>(&mut self, node: N) -> N {
        self.mutated_tree
            .get_or_insert_with(|| TreeMutator::new(node.syntax()))
            .make_mut(&node)
    }
    /// Returns a copy of the `node`, suitable for mutation.
    ///
    /// Syntax trees in rust-analyzer are typically immutable, and mutating
    /// operations panic at runtime. However, it is possible to make a copy of
    /// the tree and mutate the copy freely. Mutation is based on interior
    /// mutability, and different nodes in the same tree see the same mutations.
    ///
    /// The typical pattern for an assist is to find specific nodes in the read
    /// phase, and then get their mutable counterparts using `make_mut` in the
    /// mutable state.
    pub fn make_syntax_mut(&mut self, node: SyntaxNode) -> SyntaxNode {
        self.mutated_tree
            .get_or_insert_with(|| TreeMutator::new(&node))
            .make_syntax_mut(&node)
    }

    /// Remove specified `range` of text.
    pub fn delete(&mut self, range: TextRange) {
        self.edit.delete(range)
    }
    /// Append specified `text` at the given `offset`
    pub fn insert(&mut self, offset: TextSize, text: impl Into<String>) {
        self.edit.insert(offset, text.into())
    }
    /// Replaces specified `range` of text with a given string.
    pub fn replace(&mut self, range: TextRange, replace_with: impl Into<String>) {
        self.edit.replace(range, replace_with.into())
    }
    pub fn replace_ast<N: AstNode>(&mut self, old: N, new: N) {
        tree_diff(old.syntax(), new.syntax()).into_text_edit(&mut self.edit)
    }
    pub fn create_file(&mut self, dst: AnchoredPathBuf, content: impl Into<String>) {
        let file_system_edit = FileSystemEdit::CreateFile {
            dst,
            initial_contents: content.into(),
        };
        self.source_change.push_file_system_edit(file_system_edit);
    }
    pub fn move_file(&mut self, src: impl Into<FileId>, dst: AnchoredPathBuf) {
        let file_system_edit = FileSystemEdit::MoveFile { src: src.into(), dst };
        self.source_change.push_file_system_edit(file_system_edit);
    }

    /// Triggers the parameter hint popup after the assist is applied
    pub fn trigger_parameter_hints(&mut self) {
        self.command = Some(Command::TriggerParameterHints);
    }

    /// Renames the item at the cursor position after the assist is applied
    pub fn rename(&mut self) {
        self.command = Some(Command::Rename);
    }

    pub fn finish(mut self) -> SourceChange {
        self.commit();
        mem::take(&mut self.source_change)
    }
}

#[derive(Debug, Clone)]
pub enum FileSystemEdit {
    CreateFile {
        dst: AnchoredPathBuf,
        initial_contents: String,
    },
    MoveFile {
        src: FileId,
        dst: AnchoredPathBuf,
    },
    MoveDir {
        src: AnchoredPathBuf,
        src_id: FileId,
        dst: AnchoredPathBuf,
    },
}

impl From<FileSystemEdit> for SourceChange {
    fn from(edit: FileSystemEdit) -> SourceChange {
        SourceChange {
            source_file_edits: Default::default(),
            file_system_edits: vec![edit],
        }
    }
}
