use crate::loc::SyntaxLocFileExt;
use crate::nameres::ResolveReference;
use crate::nameres::name_resolution::{
    get_entries_from_walking_scopes, get_modules_as_entries, get_qualified_path_entries,
};
use crate::nameres::namespaces::Ns::FUNCTION;
use crate::nameres::namespaces::{FUNCTIONS, Ns};
use crate::nameres::path_kind::{PathKind, QualifiedKind, path_kind};
use crate::nameres::scope::{NamedItemsInFileExt, ScopeEntry, ScopeEntryListExt};
use crate::types::inference::InferenceCtx;
use crate::types::lowering::TyLowering;
use crate::types::ty::Ty;
use base_db::SourceDatabase;
use base_db::package_root::PackageId;
use parser::SyntaxKind::CALL_EXPR;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxNodeExt;
use syntax::ast::node_ext::syntax_node::{OptionSyntaxNodeExt, SyntaxNodeExt};
use syntax::ast::{HasItems, ReferenceElement};
use syntax::files::{InFile, InFileExt, OptionInFileExt};
use syntax::{AstNode, ast};
use vfs::FileId;

fn refine_path_expected_type(
    db: &dyn SourceDatabase,
    file_id: FileId,
    path_kind: PathKind,
    expected_type: Option<Ty>,
) -> Option<Ty> {
    let mut expected_type = expected_type;
    // if path qualifier is enum, then the expected type is that enum
    if let PathKind::Qualified { qualifier, kind, .. } = path_kind.clone() {
        match kind {
            QualifiedKind::ModuleItemOrEnumVariant | QualifiedKind::FQModuleItem => {
                let _p = tracing::debug_span!("refine expected_type").entered();
                let enum_item = qualifier
                    .reference()
                    .in_file(file_id)
                    .resolve_no_inf(db)
                    .and_then(|it| it.cast_into::<ast::Enum>(db));
                if let Some(enum_item) = enum_item {
                    expected_type = Some(Ty::new_ty_adt(enum_item.map_into()));
                    tracing::debug!("refined type {:?}", expected_type);
                }
            }
            _ => (),
        }
    }
    expected_type
}

#[tracing::instrument(level = "debug", skip_all)]
pub fn get_path_resolve_variants(
    db: &dyn SourceDatabase,
    ctx: &ResolutionContext,
    path_kind: PathKind,
) -> Vec<ScopeEntry> {
    match path_kind {
        PathKind::NamedAddress(_) | PathKind::ValueAddress(_) => {
            // no path resolution for named / value addresses
            vec![]
        }
        PathKind::NamedAddressOrUnqualifiedPath { ns, .. } | PathKind::Unqualified { ns } => {
            let mut entries = vec![];
            if ns.contains(Ns::MODULE) {
                if let Some(module) = ctx.containing_module().opt_in_file(ctx.path.file_id) {
                    // Self::call() as an expression
                    entries.push(ScopeEntry {
                        name: "Self".to_string(),
                        node_loc: module.loc(),
                        ns: Ns::MODULE,
                        scope_adjustment: None,
                    })
                }
            }
            entries.extend(get_entries_from_walking_scopes(db, ctx.path.clone(), ns));
            entries
        }

        PathKind::Qualified {
            kind: QualifiedKind::Module { address },
            ..
        } => get_modules_as_entries(db, ctx.package_id(db), address),

        PathKind::Qualified { qualifier, ns, .. } => get_qualified_path_entries(db, ctx, qualifier)
            .unwrap_or_default()
            .filter_by_ns(ns),
    }
}

#[tracing::instrument(level = "debug", skip(db, current_file_id))]
pub fn get_method_resolve_variants(
    db: &dyn SourceDatabase,
    self_ty: &Ty,
    current_file_id: FileId,
    msl: bool,
) -> Vec<ScopeEntry> {
    let package_id = db.file_package_id(current_file_id);
    let Some(InFile {
        file_id,
        value: receiver_item_module,
    }) = self_ty.adt_item_module(db, package_id)
    else {
        return vec![];
    };
    let function_entries = receiver_item_module.non_test_functions().to_entries(file_id);
    let ty_lowering = TyLowering::new(db, msl);
    let mut method_entries = vec![];
    for function_entry in function_entries {
        let Some(InFile { file_id, value: f }) = function_entry.node_loc.to_ast::<ast::Fun>(db) else {
            continue;
        };
        let Some(self_param_ty) = f
            .self_param()
            .and_then(|self_param| self_param.type_())
            .map(|self_param_type| ty_lowering.lower_type(self_param_type.in_file(file_id)))
        else {
            continue;
        };
        let self_param_with_ty_vars =
            self_param_ty.fold_ty_type_params(|ty_tp| Ty::new_ty_var_with_origin(ty_tp.origin_loc));
        let mut inference_ctx = InferenceCtx::new(db, file_id, false);
        if inference_ctx.is_tys_compatible_with_autoborrow(self_ty.clone(), self_param_with_ty_vars) {
            method_entries.push(function_entry);
        }
    }
    tracing::debug!(?method_entries);
    method_entries
}

#[tracing::instrument(
    level = "debug",
    skip(db, path, expected_type),
    fields(path = ?path.syntax_text(), file_id = ?path.file_id))]
pub fn resolve_path(
    db: &dyn SourceDatabase,
    path: InFile<ast::Path>,
    expected_type: Option<Ty>,
) -> Vec<ScopeEntry> {
    let Some(path_name) = path.value.reference_name() else {
        return vec![];
    };
    let context_element = path.clone();

    let Some(path_kind) = path_kind(path.clone().value, false) else {
        return vec![];
    };
    tracing::debug!(?path_kind);

    let ctx = ResolutionContext { path, is_completion: false };
    let entries = get_path_resolve_variants(db, &ctx, path_kind.clone());

    let entries_filtered_by_name = entries.filter_by_name(path_name.clone());
    tracing::debug!(filter_by_name = ?path_name, ?entries_filtered_by_name);

    let expected_type = refine_path_expected_type(db, ctx.path.file_id, path_kind, expected_type);
    let entries_by_expected_type = entries_filtered_by_name.filter_by_expected_type(db, expected_type);

    let entries_by_visibility = entries_by_expected_type.filter_by_visibility(db, &context_element);
    tracing::debug!(?entries_by_visibility);

    filter_by_function_namespace_special_case(entries_by_visibility, &ctx)
}

fn filter_by_function_namespace_special_case(
    entries: Vec<ScopeEntry>,
    ctx: &ResolutionContext,
) -> Vec<ScopeEntry> {
    let path_expr = ctx.parent_path_expr();
    if path_expr.is_some_and(|it| it.syntax().parent_of_type::<ast::CallExpr>().is_some()) {
        let function_entries = entries.clone().filter_by_ns(FUNCTIONS);
        return if !function_entries.is_empty() {
            function_entries
        } else {
            entries
        };
    }
    if entries.len() > 1 {
        // we're not at the callable, so drop function entries and see whether we'd get to a single entry
        let non_function_entries = entries
            .clone()
            .into_iter()
            .filter(|it| it.ns != FUNCTION)
            .collect::<Vec<_>>();
        if non_function_entries.len() == 1 {
            return non_function_entries;
        }
    }
    entries
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionContext {
    pub path: InFile<ast::Path>,
    pub is_completion: bool,
}

impl ResolutionContext {
    pub fn containing_module(&self) -> Option<ast::Module> {
        self.path.value.syntax().containing_module()
    }

    pub fn wrap_in_file<T: AstNode>(&self, node: T) -> InFile<T> {
        InFile::new(self.path.file_id, node)
    }

    pub fn parent_path_expr(&self) -> Option<ast::PathExpr> {
        self.path
            .value
            .root_path()
            .syntax()
            .parent_of_type::<ast::PathExpr>()
    }

    pub fn is_call_expr(&self) -> bool {
        let path_expr = self
            .path
            .value
            .root_path()
            .syntax()
            .parent_of_type::<ast::PathExpr>();
        path_expr.is_some_and(|it| it.syntax().parent().is_kind(CALL_EXPR))
    }

    pub fn package_id(&self, db: &dyn SourceDatabase) -> PackageId {
        db.file_package_id(self.path.file_id)
    }
}
