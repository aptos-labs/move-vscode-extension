use crate::db::HirDatabase;
use crate::nameres::namespaces::{Ns, NsSetExt, TYPES_N_ENUMS};
use crate::nameres::paths;
use crate::nameres::scope::ScopeEntry;
use crate::node_ext::ModuleLangExt;
use crate::InFile;
use base_db::SourceRootDatabase;
use parser::SyntaxKind::MODULE;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxNodeExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::ast::visibility::{Vis, VisLevel};
use syntax::ast::{HasAttrs, HasReference, HasVisibility, NamedItemScope};
use syntax::{ast, unwrap_or_continue, AstNode};

pub fn is_visible_in_context(
    db: &dyn HirDatabase,
    scope_entry: &ScopeEntry,
    context: &InFile<impl HasReference>,
) -> bool {
    use syntax::SyntaxKind::*;

    let InFile {
        file_id: context_file_id,
        value: context,
    } = context;
    // inside msl everything is visible
    if context.syntax().is_msl_context() {
        return true;
    }

    // if inside MvAttrItem like abort_code=
    if context.syntax().ancestor_strict::<ast::AttrItem>().is_some() {
        return true;
    }

    let Some(InFile {
        file_id: item_file_id,
        value: item,
    }) = scope_entry.node_loc.cast::<ast::AnyHasName>(db.upcast())
    else {
        return false;
    };
    let item_kind = item.syntax().kind();
    let item_ns = scope_entry.ns;
    let opt_visible_item = ast::AnyHasVisibility::cast(item.syntax().clone());

    let context_usage_scope = context.syntax().item_scope();
    let context_opt_path = ast::Path::cast(context.syntax().to_owned());
    if let Some(path) = context_opt_path.clone() {
        if path.use_speck().is_some() {
            if item_kind == MODULE {
                return true;
            }
            // for use specks, items needs to be public to be visible, no other rules apply
            if let Some(visible_item) = opt_visible_item.clone() {
                if visible_item.vis() != Vis::Private {
                    return true;
                }
            }

            // msl-only items are available from imports
            if item.syntax().is_msl_only_item() {
                return true;
            }

            // consts are importable in tests
            if context_usage_scope.is_test() && item_ns.contains(Ns::NAME) {
                return true;
            }
        }
    }

    // #[test] functions cannot be used from non-imports
    if item.syntax().kind() == FUN {
        if ast::Fun::cast(item.syntax().clone()).unwrap().has_atom_attr("test") {
            return false;
        }
    }

    let item_module = item.syntax().containing_module();
    // 0x0::builtins module items are always visible
    if item_module.is_some() && item_module.clone().unwrap().is_builtins() {
        return true;
    }

    let item_scope = match scope_entry.scope_adjustment {
        Some(adjustment) => item.syntax().item_scope().shrink_scope(adjustment),
        None => item.syntax().item_scope(),
    };
    // i.e. #[test_only] items in non-test-only scope
    if item_scope != NamedItemScope::Main {
        // cannot be used everywhere, need to check for scope compatibility
        if item_scope != context_usage_scope {
            return false;
        }
    }

    // we're in non-msl scope at this point, msl only items aren't accessible
    if item.syntax().is_msl_only_item() {
        return false;
    }

    // local methods, Self::method - everything is visible
    let context_module = context.syntax().containing_module();
    if item_module.is_some() && context_module.is_some() && item_module == context_module {
        return true;
    }

    // item is type, check whether it's allowed in the context
    if item_ns.contains_any_of(TYPES_N_ENUMS) {
        let root_path = context_opt_path.map(|path| path.root_path());
        if let Some(root_path) = root_path {
            // todo: add more checks when structs/enums will have visibility
            if root_path.syntax().kind() == PATH_TYPE {
                return true;
            }
        }
    }

    let vis = opt_visible_item.map(|f| f.vis()).unwrap_or(Vis::Public);
    match vis {
        Vis::Private => false,
        Vis::Public => true,
        Vis::Restricted(vis_level) => match vis_level {
            VisLevel::Friend => {
                if let (Some(item_module), Some(context_module)) = (item_module, context_module) {
                    let friend_decls = item_module.friend_decls();
                    for friend_decl in friend_decls {
                        let friend_path = unwrap_or_continue!(friend_decl.path());
                        if let Some(friend_entry) =
                            paths::resolve_single(db, InFile::new(item_file_id, friend_path))
                        {
                            let friend_module = unwrap_or_continue!(friend_entry
                                .node_loc
                                .cast::<ast::Module>(db.upcast()));
                            if friend_module.value == context_module {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            VisLevel::Package => {
                // check for the same source root
                // todo: change later to package_id
                db.file_source_root(*context_file_id) == db.file_source_root(item_file_id)
            }
        },
    }
}
