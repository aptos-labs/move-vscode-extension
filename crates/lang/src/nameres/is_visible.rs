use crate::db::HirDatabase;
use crate::nameres::namespaces::{Ns, NsSetExt, TYPES_N_ENUMS};
use crate::nameres::scope::ScopeEntry;
use crate::node_ext::ModuleLangExt;
use crate::InFile;
use parser::SyntaxKind::MODULE;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxNodeExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::ast::visibility::{Vis, VisLevel};
use syntax::ast::{HasAttrs, HasReference, NamedItemScope};
use syntax::{ast, AstNode};

pub fn is_visible_in_context(
    db: &dyn HirDatabase,
    scope_entry: &ScopeEntry,
    context: &impl HasReference,
) -> bool {
    use syntax::SyntaxKind::*;

    // inside msl everything is visible
    if context.syntax().is_msl_context() {
        return true;
    }

    // if inside MvAttrItem like abort_code=
    if context.syntax().ancestor_strict::<ast::AttrItem>().is_some() {
        return true;
    }

    let Some(InFile {
        file_id: _,
        value: item,
    }) = scope_entry.named_node_loc.cast::<ast::AnyHasName>(db.upcast())
    else {
        return false;
    };
    let item_kind = item.syntax().kind();
    let item_ns = scope_entry.ns;
    let opt_fun = ast::Fun::cast(item.syntax().clone());

    let context_usage_scope = context.syntax().item_scope();
    let context_opt_path = ast::Path::cast(context.syntax().to_owned());
    if let Some(path) = context_opt_path.clone() {
        if path.use_speck().is_some() {
        // if let Some(use_speck) = path.use_speck() {
            if item_kind == MODULE {
                return true;
            }
            // for use specks, items needs to be public to be visible, no other rules apply
            // todo: add other types of funs
            if let Some(fun) = opt_fun.clone() {
                if fun.vis() != Vis::Private {
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
    if let Some(fun) = opt_fun.clone() {
        if fun.has_atom_attr("test") {
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

    if let Some(vis) = opt_fun.map(|f| f.vis()) {
        return match vis {
            Vis::Private => false,
            Vis::Public => true,
            Vis::Restricted(vis_level) => match vis_level {
                VisLevel::Friend => {
                    if let (Some(item_module), Some(context_module)) = (item_module, context_module) {
                        // todo: resolve friend modules
                    }
                    false
                }
                VisLevel::Package => {
                    // todo: check packages equality
                    false
                }
            },
        }
    }

    true
}
