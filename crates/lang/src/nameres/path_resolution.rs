use crate::db::HirDatabase;
use crate::loc::SyntaxLocFileExt;
use crate::nameres::ResolveReference;
use crate::nameres::name_resolution::{
    get_entries_from_walking_scopes, get_modules_as_entries, get_qualified_path_entries,
};
use crate::nameres::namespaces::{FUNCTIONS, Ns};
use crate::nameres::path_kind::{PathKind, QualifiedKind, path_kind};
use crate::nameres::scope::{NamedItemsInFileExt, ScopeEntry, ScopeEntryListExt};
use crate::types::inference::InferenceCtx;
use crate::types::lowering::TyLowering;
use crate::types::ty::Ty;
use base_db::package_root::PackageRootId;
use parser::SyntaxKind::CALL_EXPR;
use syntax::ast::HasItems;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxNodeExt;
use syntax::ast::node_ext::syntax_node::OptionSyntaxNodeExt;
use syntax::files::{InFile, InFileExt, OptionInFileExt};
use syntax::{AstNode, ast};
use vfs::FileId;

pub fn get_path_resolve_variants_with_expected_type(
    db: &dyn HirDatabase,
    ctx: &ResolutionContext,
    path_kind: PathKind,
    expected_type: Option<Ty>,
) -> Vec<ScopeEntry> {
    let mut expected_type = expected_type;

    // if path qualifier is enum, then the expected type is that enum
    if let PathKind::Qualified { qualifier, kind, .. } = path_kind.clone() {
        match kind {
            QualifiedKind::ModuleItemOrEnumVariant | QualifiedKind::FQModuleItem => {
                let enum_item = qualifier
                    .reference()
                    .in_file(ctx.path.file_id)
                    .resolve_no_inf(db)
                    .and_then(|it| it.cast_into::<ast::Enum>(db));
                if let Some(enum_item) = enum_item {
                    expected_type = Some(Ty::new_ty_adt(enum_item.in_file_into()));
                }
            }
            _ => (),
        }
    }

    let path_entries = get_path_resolve_variants(db, ctx, path_kind);
    path_entries.filter_by_expected_type(db, expected_type)
}

pub fn get_path_resolve_variants(
    db: &dyn HirDatabase,
    ctx: &ResolutionContext,
    path_kind: PathKind,
) -> Vec<ScopeEntry> {
    match path_kind {
        PathKind::Unknown => vec![],
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
        } => get_modules_as_entries(db, ctx.package_root_id(db), address),

        PathKind::Qualified { qualifier, ns, .. } => get_qualified_path_entries(db, ctx, qualifier)
            .unwrap_or_default()
            .filter_by_ns(ns),
    }
}

#[tracing::instrument(level = "debug", skip(db, current_file_id))]
pub fn get_method_resolve_variants(
    db: &dyn HirDatabase,
    self_ty: &Ty,
    current_file_id: FileId,
) -> Vec<ScopeEntry> {
    let package_id = db.file_package_root_id(current_file_id);
    let Some(InFile {
        file_id,
        value: receiver_item_module,
    }) = self_ty.adt_item_module(db, package_id)
    else {
        return vec![];
    };
    let function_entries = receiver_item_module
        .non_test_functions()
        .to_in_file_entries(file_id);
    let ty_lowering = TyLowering::new(db);
    let mut method_entries = vec![];
    for function_entry in function_entries {
        let Some(InFile { file_id, value: f }) = function_entry.node_loc.to_ast::<ast::Fun>(db.upcast())
        else {
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
    skip(db, path),
    fields(path = ?path.syntax_text(), file_id = ?path.file_id))]
pub fn resolve_path(
    db: &dyn HirDatabase,
    path: InFile<ast::Path>,
    expected_type: Option<Ty>,
) -> Vec<ScopeEntry> {
    let Some(path_name) = path.value.reference_name() else {
        return vec![];
    };
    let context_element = path.clone();

    let path_kind = path_kind(path.clone().value, false);
    tracing::debug!(?path_kind);

    let ctx = ResolutionContext {
        path,
        is_completion: false,
    };
    let entries = get_path_resolve_variants_with_expected_type(db, &ctx, path_kind, expected_type);
    tracing::debug!(?entries);

    let entries_filtered_by_name = entries.filter_by_name(path_name.clone());
    tracing::debug!(?path_name, ?entries_filtered_by_name);

    let final_entries = entries_filtered_by_name.filter_by_visibility(db, &context_element);

    if ctx.is_call_expr() {
        let function_entries = final_entries.clone().filter_by_ns(FUNCTIONS);

        return if !function_entries.is_empty() {
            function_entries
        } else {
            final_entries
        };
    }

    final_entries
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

    pub fn is_call_expr(&self) -> bool {
        self.path.value.root_path().syntax().parent().is_kind(CALL_EXPR)
    }

    pub fn package_root_id(&self, db: &dyn HirDatabase) -> PackageRootId {
        db.file_package_root_id(self.path.file_id)
    }
}
