mod source_to_def;

use crate::hir_db::inference_loc;
use crate::loc::{SyntaxLocFileExt, SyntaxLocInput};
use crate::nameres;
use crate::nameres::fq_named_element::{ItemFQName, ItemFQNameOwner};
use crate::nameres::scope::{ScopeEntry, VecExt};
use crate::node_ext::item::ModuleItemExt;
use crate::semantics::source_to_def::SourceToDefCache;
use crate::types::inference::InferenceCtx;
use crate::types::inference::inference_result::InferenceResult;
use crate::types::lowering::TyLowering;
use crate::types::ty::Ty;
use base_db::inputs::InternFileId;
use base_db::package_root::PackageId;
use base_db::{SourceDatabase, source_db};
use itertools::{Itertools, repeat_n};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::{fmt, ops};
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::{FileRange, InFile};
use syntax::{AstNode, SyntaxNode, SyntaxToken, ast};
use vfs::FileId;

const MAX_FILE_ID: u32 = 0x7fff_ffff;

/// Primary API to get semantic information, like types, from syntax trees.
pub struct Semantics<'db, DB> {
    pub db: &'db DB,
    imp: SemanticsImpl<'db>,
}

pub struct SemanticsImpl<'db> {
    db: &'db dyn SourceDatabase,
    ws_root: PackageId,
    s2d_cache: RefCell<SourceToDefCache>,
    inference_cache: RefCell<HashMap<(SyntaxLocInput<'db>, bool), Arc<InferenceResult>>>,
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

impl<DB: SourceDatabase> Semantics<'_, DB> {
    pub fn new(db: &DB, ws_file_id: FileId) -> Semantics<'_, DB> {
        // tracing::debug!("db_revision = {:?}", salsa::plumbing::current_revision(db));
        let ws_root = db.file_package_id(ws_file_id);
        let impl_ = SemanticsImpl::new(db, ws_root);
        // add builtins file to cache
        if let Some(builtins_file_id) = db.builtins_file_id() {
            impl_.parse(builtins_file_id.data(db));
        }
        Semantics { db, imp: impl_ }
    }
}

impl<'db> SemanticsImpl<'db> {
    fn new(db: &'db dyn SourceDatabase, ws_root: PackageId) -> Self {
        SemanticsImpl {
            db,
            ws_root,
            s2d_cache: Default::default(),
            inference_cache: Default::default(),
        }
    }

    pub fn parse(&self, file_id: FileId) -> ast::SourceFile {
        let tree = source_db::parse(self.db, file_id.intern(self.db)).tree();
        self.cache(tree.syntax().clone(), file_id);
        tree
    }

    pub fn resolve_to_element<Named: ast::NamedElement>(
        &self,
        reference: InFile<ast::ReferenceElement>,
    ) -> Option<InFile<Named>> {
        let element = self
            .resolve_in_file(reference)
            .single_or_none()?
            .cast_into::<Named>(self.db)?;
        // cache file_id
        self.parse(element.file_id);
        Some(element)
    }

    pub fn resolve(&self, reference: ast::ReferenceElement) -> Vec<ScopeEntry> {
        let reference = self.wrap_node_infile(reference);
        self.resolve_in_file(reference)
    }

    pub fn resolve_in_file(&self, reference: InFile<ast::ReferenceElement>) -> Vec<ScopeEntry> {
        let msl = reference.syntax().value.is_msl_context();
        let inference = self.inference(&reference, msl);
        nameres::resolve_multi(self.db, reference, inference).unwrap_or_default()
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

    pub fn render_ty(&self, ty: &Ty) -> String {
        ty.render(self.db, None)
    }

    pub fn render_ty_truncated(&self, ty: &Ty, context_file_id: FileId) -> String {
        ty.render(self.db, Some(context_file_id))
    }

    pub fn render_ty_expected_form(&self, ty: &Ty) -> String {
        match ty {
            Ty::Tuple(ty_tuple) => {
                let arity = ty_tuple.types.len();
                let expected_form = repeat_n("_", arity).join(", ");
                format!("tuple binding of length {arity}: ({expected_form})")
            }
            Ty::Adt(ty_adt) => {
                format!(
                    "struct binding of type '{}'",
                    self.render_ty(&Ty::Adt(ty_adt.clone()))
                )
            }
            _ => "a single variable".to_string(),
        }
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

    pub fn inference<T: AstNode>(&self, node: &InFile<T>, msl: bool) -> Option<Arc<InferenceResult>> {
        let ctx_owner = node.and_then_ref(|it| it.syntax().inference_ctx_owner())?;

        let owner_loc = SyntaxLocInput::new(self.db, ctx_owner.loc());
        let cache_key = (owner_loc, msl);

        let mut cache = self.inference_cache.borrow_mut();
        if cache.contains_key(&cache_key) {
            return Some(Arc::clone(cache.get(&cache_key).unwrap()));
        }

        let inf = inference_loc(self.db, owner_loc, msl);
        cache.insert(cache_key, Arc::clone(&inf));

        Some(inf)
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

    #[allow(unused)]
    fn wrap_token_infile(&self, token: SyntaxToken) -> InFile<SyntaxToken> {
        let (file_id, _) = self.find_file(&token.parent().unwrap()).unpack();
        InFile::new(file_id, token)
    }

    /// Attempts to map the node out of macro expanded files returning the original file range.
    /// If upmapping is not possible, this will fall back to the range of the macro call of the
    /// macro file the node resides in.
    pub fn file_range(&self, node: &SyntaxNode) -> FileRange {
        let (file_id, node) = self.find_file(node).unpack();
        FileRange {
            file_id,
            range: node.text_range(),
        }
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
