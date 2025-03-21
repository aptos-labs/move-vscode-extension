mod source_to_def;

use crate::db::HirDatabase;
use crate::nameres::scope::ScopeEntry;
use crate::semantics::source_to_def::SourceToDefCache;
use crate::InFile;
use std::cell::RefCell;
use std::{fmt, ops};
use syntax::{ast, AstNode, SyntaxNode, SyntaxToken};
use vfs::FileId;

/// Primary API to get semantic information, like types, from syntax trees.
pub struct Semantics<'db, DB> {
    pub db: &'db DB,
    imp: SemanticsImpl<'db>,
}

pub struct SemanticsImpl<'db> {
    pub db: &'db dyn HirDatabase,
    s2d_cache: RefCell<SourceToDefCache>,
}

impl<DB> fmt::Debug for Semantics<'_, DB> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Semantics {{ ... }}")
    }
}

impl<'db, DB> ops::Deref for Semantics<'db, DB> {
    type Target = SemanticsImpl<'db>;

    fn deref(&self) -> &Self::Target {
        &self.imp
    }
}

impl<DB: HirDatabase> Semantics<'_, DB> {
    pub fn new(db: &DB) -> Semantics<'_, DB> {
        let impl_ = SemanticsImpl::new(db);
        Semantics { db, imp: impl_ }
    }
}

impl<'db> SemanticsImpl<'db> {
    fn new(db: &'db dyn HirDatabase) -> Self {
        SemanticsImpl {
            db,
            s2d_cache: Default::default(),
        }
    }

    pub fn parse(&self, file_id: FileId) -> ast::SourceFile {
        let tree = self.db.parse(file_id).tree();
        self.cache(tree.syntax().clone(), file_id);
        tree
    }

    pub fn resolve_reference(&self, reference: ast::AnyReferenceElement) -> Option<ScopeEntry> {
        let reference = self.wrap_node_infile(reference);
        self.db.resolve(reference)
    }

    // todo: rename to root_file_id()
    fn lookup(&self, root_node: &SyntaxNode) -> Option<FileId> {
        let cache = self.s2d_cache.borrow();
        cache.root_to_file_cache.get(root_node).copied()
    }

    fn wrap_node_infile<N: AstNode>(&self, node: N) -> InFile<N> {
        let InFile { file_id, .. } = self.find_file(node.syntax());
        InFile::new(file_id, node)
    }

    fn wrap_token_infile(&self, token: SyntaxToken) -> InFile<SyntaxToken> {
        let InFile { file_id, .. } = self.find_file(&token.parent().unwrap());
        InFile::new(file_id, token)
    }

    /// Wraps the node in a [`InFile`] with the file id it belongs to.
    fn find_file<'node>(&self, node: &'node SyntaxNode) -> InFile<&'node SyntaxNode> {
        let root_node = find_root(node);
        let file_id = self.lookup(&root_node).unwrap_or_else(|| {
            panic!(
                "\n\nFailed to lookup {:?} in this Semantics.\n\
                 Make sure to only query nodes derived from this instance of Semantics.\n\
                 root node:   {:?}\n\
                 known nodes: {}\n\n",
                node,
                root_node,
                self.s2d_cache
                    .borrow()
                    .root_to_file_cache
                    .keys()
                    .map(|it| format!("{it:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        });
        InFile::new(file_id, node)
    }

    fn cache(&self, root_node: SyntaxNode, file_id: FileId) {
        SourceToDefCache::cache(
            &mut self.s2d_cache.borrow_mut().root_to_file_cache,
            root_node,
            file_id,
        );
    }
}

fn find_root(node: &SyntaxNode) -> SyntaxNode {
    node.ancestors().last().unwrap()
}
