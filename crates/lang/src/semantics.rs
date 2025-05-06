mod source_to_def;

use crate::HirDatabase;
use crate::db::NodeInferenceExt;
use crate::nameres::ResolveReference;
use crate::nameres::fq_named_element::{ItemFQName, ItemFQNameOwner};
use crate::nameres::scope::ScopeEntry;
use crate::node_ext::item::ModuleItemExt;
use crate::semantics::source_to_def::SourceToDefCache;
use crate::types::inference::InferenceCtx;
use crate::types::inference::inference_result::InferenceResult;
use crate::types::lowering::TyLowering;
use crate::types::ty::Ty;
use base_db::inputs::InternFileId;
use base_db::package_root::PackageRootId;
use std::cell::RefCell;
use std::sync::Arc;
use std::{fmt, ops};
use syntax::files::InFile;
use syntax::{AstNode, SyntaxNode, SyntaxToken, ast};
use vfs::FileId;

const MAX_FILE_ID: u32 = 0x7fff_ffff;

/// Primary API to get semantic information, like types, from syntax trees.
pub struct Semantics<'db> {
    imp: SemanticsImpl<'db>,
}

pub struct SemanticsImpl<'db> {
    db: &'db dyn HirDatabase,
    ws_root: PackageRootId,
    s2d_cache: RefCell<SourceToDefCache>,
}

impl fmt::Debug for Semantics<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Semantics {{ ... }}")
    }
}

impl<'db> ops::Deref for Semantics<'db> {
    type Target = SemanticsImpl<'db>;

    fn deref(&self) -> &Self::Target {
        &self.imp
    }
}

impl Semantics<'_> {
    pub fn new(db: &dyn HirDatabase, ws_file_id: FileId) -> Semantics<'_> {
        let ws_root = db.file_package_root(ws_file_id).data(db);
        let impl_ = SemanticsImpl::new(db, ws_root);
        // add builtins file to cache
        if let Some(builtins_file_id) = db.builtins_file_id() {
            impl_.parse(builtins_file_id.data(db));
        }
        Semantics { imp: impl_ }
    }
}

impl<'db> SemanticsImpl<'db> {
    fn new(db: &'db dyn HirDatabase, ws_root: PackageRootId) -> Self {
        SemanticsImpl {
            db,
            ws_root,
            s2d_cache: Default::default(),
        }
    }

    pub fn parse(&self, file_id: FileId) -> ast::SourceFile {
        let tree = self.db.parse(file_id.intern(self.db)).tree();
        self.cache(tree.syntax().clone(), file_id);
        tree
    }

    pub fn resolve(&self, reference: ast::AnyReferenceElement) -> Vec<ScopeEntry> {
        let reference = self.wrap_node_infile(reference);
        self.resolve_in_file(reference)
    }

    pub fn resolve_in_file(&self, reference: InFile<ast::AnyReferenceElement>) -> Vec<ScopeEntry> {
        reference.resolve_multi(self.db).unwrap_or_default()
    }

    pub fn resolve_to_element<N: ast::NamedElement>(
        &self,
        reference: InFile<ast::AnyReferenceElement>,
    ) -> Option<InFile<N>> {
        let scope_entry = reference.resolve(self.db);
        let element = scope_entry?.cast_into::<N>(self.db)?;
        // cache file_id
        self.parse(element.file_id);
        Some(element)
    }

    pub fn fun_module(&self, fun: InFile<ast::AnyFun>) -> Option<InFile<ast::Module>> {
        fun.module(self.db)
    }

    pub fn get_expr_type(&self, expr: &InFile<ast::Expr>, msl: bool) -> Option<Ty> {
        let inference = self.inference(expr, msl)?;
        inference.get_expr_type(&expr.value)
    }

    pub fn get_ident_pat_type(&self, ident_pat: &InFile<ast::IdentPat>, msl: bool) -> Option<Ty> {
        let inference = self.inference(ident_pat, msl)?;
        inference.get_pat_type(&ast::Pat::IdentPat(ident_pat.value.clone()))
    }

    pub fn render_ty(&self, ty: Ty) -> String {
        ty.render(self.db)
    }

    pub fn fq_name(&self, item: impl AstNode) -> Option<ItemFQName> {
        let item = self.wrap_node_infile(item);
        item.fq_name(self.db)
    }

    /// returns module for the Ty inner item, for the tys where is makes sense
    pub fn ty_module(&self, ty: &Ty) -> Option<ast::Module> {
        ty.adt_item_module(self.db, self.ws_root).map(|it| it.value)
    }

    pub fn lower_type(&self, type_: InFile<ast::Type>, msl: bool) -> Ty {
        TyLowering::new(self.db, msl).lower_type(type_)
    }

    pub fn is_tys_compatible(&self, left_ty: Ty, right_ty: Ty, with_autoborrow: bool) -> bool {
        // Any file_id could be used here, we are not interested in unification. Could be improved later.
        let ctx = &mut InferenceCtx::new(self.db, FileId::from_raw(MAX_FILE_ID), false);
        if with_autoborrow {
            ctx.is_tys_compatible_with_autoborrow(left_ty, right_ty)
        } else {
            ctx.is_tys_compatible(left_ty, right_ty)
        }
    }

    fn inference<T: AstNode>(&self, node: &InFile<T>, msl: bool) -> Option<Arc<InferenceResult>> {
        node.inference(self.db, msl)
    }

    pub fn wrap_node_infile<N: AstNode>(&self, node: N) -> InFile<N> {
        let (file_id, _) = self.find_file(node.syntax()).unpack();
        InFile::new(file_id, node)
    }

    // todo: rename to root_file_id()
    fn lookup(&self, root_node: &SyntaxNode) -> Option<FileId> {
        let cache = self.s2d_cache.borrow();
        cache.root_to_file_cache.get(root_node).copied()
    }

    fn wrap_token_infile(&self, token: SyntaxToken) -> InFile<SyntaxToken> {
        let (file_id, _) = self.find_file(&token.parent().unwrap()).unpack();
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
