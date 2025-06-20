use crate::item_scope::NamedItemScope;
use crate::loc::{SyntaxLocFileExt, SyntaxLocNodeExt};
use crate::nameres::namespaces::{Ns, TYPES_N_ENUMS};
use crate::nameres::scope::ScopeEntry;
use crate::node_ext::ModuleLangExt;
use crate::{hir_db, nameres};
use base_db::SourceDatabase;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::ast::visibility::{Vis, VisLevel};
use syntax::ast::{HasAttrs, HasVisibility};
use syntax::files::{InFile, InFileExt, OptionInFileExt};
use syntax::{AstNode, ast};

pub fn is_visible_in_context(
    db: &dyn SourceDatabase,
    scope_entry: &ScopeEntry,
    context: &InFile<ast::ReferenceElement>,
) -> bool {
    use syntax::SyntaxKind::*;

    let (context_file_id, context) = context.unpack_ref();

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
    }) = scope_entry.node_loc.to_ast::<ast::NamedElement>(db)
    else {
        return false;
    };
    let item_kind = item.syntax().kind();
    let item_ns = scope_entry.ns;
    let opt_visible_item = ast::AnyHasVisibility::cast(item.syntax().clone());

    let context_usage_scope = hir_db::item_scope(db, context.loc(context_file_id));
    let context_opt_path = ast::Path::cast(context.syntax().to_owned());
    if let Some(path) = context_opt_path.clone() {
        if path.root_parent_of_type::<ast::UseSpeck>().is_some() {
            // those are always public in use specks
            if matches!(item_kind, MODULE | STRUCT | ENUM) {
                return true;
            }

            // items needs to be non-public to be visible, no other rules apply in use specks
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
            if context_usage_scope.is_test() && item_ns == Ns::NAME {
                return true;
            }
        }
    }

    // #[test] functions cannot be used from non-imports
    if let Some(fun) = ast::Fun::cast(item.syntax().clone()) {
        if fun.has_atom_attr("test") {
            return false;
        }
    }

    let item_module = item.syntax().containing_module();
    // 0x0::builtins module functions are always visible
    if item.syntax().kind() == FUN && item_module.clone().is_some_and(|m| m.is_builtins()) {
        return true;
    }

    let item_loc = item.clone().in_file(item_file_id).loc();

    let mut item_scope = hir_db::item_scope(db, item_loc);
    if let Some(adjustment) = scope_entry.scope_adjustment {
        item_scope = item_scope.shrink_scope(adjustment);
    }
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

    // local items in script
    if let Some(context_script) = context.syntax().containing_script() {
        if item
            .syntax()
            .containing_script()
            .is_some_and(|it| context_script == it)
        {
            return true;
        }
    }

    // item is type, check whether it's allowed in the context
    if TYPES_N_ENUMS.contains(item_ns) {
        let opt_path_parent = context_opt_path
            .map(|path| path.root_path())
            .and_then(|it| it.syntax().parent());
        if let Some(path_parent) = opt_path_parent {
            if path_parent.kind() == PATH_TYPE {
                return true;
            }
        }
    }
    let vis = opt_visible_item
        .map(|visible_item| visible_item.vis())
        .unwrap_or(Vis::Public);
    match vis {
        Vis::Private => false,
        Vis::Public => true,
        Vis::Restricted(vis_level) => match vis_level {
            VisLevel::Friend => {
                if let (Some(item_module), Some(context_module)) = (item_module, context_module) {
                    let friend_decls = item_module.friend_decls();
                    for friend_decl in friend_decls {
                        let friend_path = friend_decl.path().opt_in_file(item_file_id);
                        let Some(friend_module) = friend_path
                            .and_then(|path| nameres::resolve_no_inf_cast::<ast::Module>(db, path))
                        else {
                            continue;
                        };
                        if friend_module.value == context_module {
                            return true;
                        }
                    }
                }
                false
            }
            VisLevel::Package => {
                // check for the same source root
                db.file_package_id(context_file_id) == db.file_package_id(item_file_id)
            }
        },
    }
}
