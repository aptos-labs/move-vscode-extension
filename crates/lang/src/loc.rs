use crate::item_scope::NamedItemScope;
use base_db::inputs::InternFileId;
use base_db::{SourceDatabase, source_db};
use parser::SyntaxKind;
use std::fmt;
use std::fmt::Formatter;
use syntax::algo::ancestors_at_offset;
use syntax::files::InFile;
use syntax::{AstNode, NodeOrToken, SourceFile, SyntaxNodeOrToken, TextRange, TextSize, ast};
use vfs::FileId;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct SyntaxLoc {
    file_id: FileId,
    text_range: TextRange,
    kind: SyntaxKind,
    // only for debugging here, might be removed in the future
    node_name: Option<String>,
}

impl SyntaxLoc {
    pub fn from_ast_node<T: AstNode>(file_id: FileId, node: &T) -> Self {
        let node_or_token = SyntaxNodeOrToken::Node(node.syntax().clone());
        Self::from_node_or_token(file_id, node_or_token)
    }

    pub fn from_node_or_token(file_id: FileId, node_or_token: SyntaxNodeOrToken) -> Self {
        let text_range = node_or_token.text_range();
        let kind = node_or_token.kind();

        let mut node_name = None;
        if let SyntaxNodeOrToken::Node(node) = node_or_token {
            node_name = node
                .children_with_tokens()
                .find(|child| {
                    let kind = child.kind();
                    kind == SyntaxKind::NAME
                        || kind == SyntaxKind::NAME_REF
                        || kind == SyntaxKind::PATH_SEGMENT
                        || kind == SyntaxKind::QUOTE_IDENT
                })
                .map(|it| it.to_string());
        }

        SyntaxLoc {
            file_id: file_id.to_owned(),
            text_range,
            kind,
            node_name,
        }
    }

    pub fn to_ast<T: AstNode>(&self, db: &dyn SourceDatabase) -> Option<InFile<T>> {
        let file = self.get_source_file(db)?;
        let ancestors_at_offset = ancestors_at_offset(file.syntax(), self.node_offset());
        for ancestor in ancestors_at_offset {
            if ancestor.text_range().end() == self.node_offset() {
                if let Some(node) = T::cast(ancestor) {
                    return Some(InFile::new(self.file_id, node));
                }
            }
        }
        None
    }

    pub fn item_scope(&self, db: &dyn SourceDatabase) -> Option<NamedItemScope> {
        use syntax::SyntaxKind::*;

        let file = self.get_source_file(db)?;
        let ancestors = ancestors_at_offset(file.syntax(), self.node_offset());
        for ancestor in ancestors {
            let Some(has_attrs) = ast::AnyHasAttrs::cast(ancestor.clone()) else {
                continue;
            };
            if matches!(
                ancestor.kind(),
                SCHEMA | ITEM_SPEC | MODULE_SPEC | SPEC_BLOCK_EXPR
            ) {
                return Some(NamedItemScope::Verify);
            }
            if let Some(ancestor_scope) = item_scope_from_attributes(has_attrs) {
                return Some(ancestor_scope);
            }
        }
        Some(NamedItemScope::Main)
    }

    pub fn file_id(&self) -> FileId {
        self.file_id
    }

    pub fn kind(&self) -> SyntaxKind {
        self.kind
    }

    pub fn text_range(&self) -> TextRange {
        self.text_range
    }

    pub fn node_offset(&self) -> TextSize {
        self.text_range.end()
    }

    pub fn node_name(&self) -> Option<String> {
        self.node_name.to_owned()
    }

    fn get_source_file(&self, db: &dyn SourceDatabase) -> Option<SourceFile> {
        let file = source_db::parse(db, self.file_id.intern(db)).tree();
        if !file.syntax().text_range().contains_inclusive(self.node_offset()) {
            tracing::error!(
                "stale cache error: {:?} is outside of the file range {:?}",
                self,
                file.syntax().text_range()
            );
            return None;
        }
        Some(file)
    }
}

impl fmt::Debug for SyntaxLoc {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.node_name {
            Some(name) => f
                .debug_tuple("Loc")
                .field(&format!(
                    "{:?} named '{}' at {}::{:?}",
                    self.kind,
                    name,
                    self.file_id.index(),
                    self.node_offset()
                ))
                .finish(),
            None => f
                .debug_tuple("Loc")
                .field(&format!(
                    "{:?} at {}::{:?}",
                    self.kind,
                    self.file_id.index(),
                    self.node_offset()
                ))
                .finish(),
        }
    }
}

#[salsa_macros::interned(debug)]
pub struct SyntaxLocInput {
    pub syntax_loc: SyntaxLoc,
}

impl SyntaxLocInput<'_> {
    pub fn to_ast<T: AstNode>(&self, db: &dyn SourceDatabase) -> Option<InFile<T>> {
        self.syntax_loc(db).to_ast(db)
    }
}

pub trait SyntaxLocFileExt {
    fn loc(&self) -> SyntaxLoc;
}

impl<T: AstNode> SyntaxLocFileExt for InFile<T> {
    fn loc(&self) -> SyntaxLoc {
        SyntaxLoc::from_ast_node(self.file_id, &self.value)
    }
}

pub trait SyntaxLocNodeExt {
    fn loc(&self, file_id: FileId) -> SyntaxLoc;
}

impl<T: AstNode> SyntaxLocNodeExt for T {
    fn loc(&self, file_id: FileId) -> SyntaxLoc {
        SyntaxLoc::from_ast_node(file_id, self)
    }
}

fn item_scope_from_attributes(has_attrs: impl ast::HasAttrs) -> Option<NamedItemScope> {
    if has_attrs.has_atom_attr("test_only") || has_attrs.has_atom_attr("test") {
        return Some(NamedItemScope::Test);
    }
    if has_attrs.has_atom_attr("verify_only") {
        return Some(NamedItemScope::Verify);
    }
    None
}
