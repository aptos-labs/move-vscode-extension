// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::item_scope::ItemScope;
use crate::loc::{SyntaxLoc, SyntaxLocFileExt};
use crate::nameres::namespaces::{Ns, TYPES_N_ENUMS};
use crate::nameres::scope::ScopeEntry;
use crate::node_ext::ModuleLangExt;
use crate::{hir_db, nameres};
use base_db::SourceDatabase;
use std::vec::IntoIter;
use syntax::ast::HasVisibility;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::ast::visibility::{Vis, VisLevel};
use syntax::files::{InFile, InFileExt, OptionInFileExt};
use syntax::{AstNode, SyntaxElement, ast};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ItemInvisibleReason {
    Private { vis: Vis },
    WrongItemScope { item_scope: ItemScope },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScopeEntryWithVis {
    pub scope_entry: ScopeEntry,
    pub invis_reason: Option<ItemInvisibleReason>,
}

impl ScopeEntry {
    pub fn into_always_visible(self) -> ScopeEntryWithVis {
        ScopeEntryWithVis {
            scope_entry: self,
            invis_reason: None,
        }
    }
}

pub trait ScopeEntryWithVisExt {
    fn entries(self) -> IntoIter<ScopeEntryWithVis>;

    fn into_visible_entries(self) -> Vec<ScopeEntry>
    where
        Self: Sized,
    {
        self.entries()
            .filter_map(|it| it.invis_reason.is_none().then_some(it.scope_entry))
            .collect()
    }
}

impl ScopeEntryWithVisExt for Vec<ScopeEntryWithVis> {
    fn entries(self) -> IntoIter<ScopeEntryWithVis> {
        self.into_iter()
    }
}
impl ScopeEntryWithVisExt for IntoIter<ScopeEntryWithVis> {
    fn entries(self) -> IntoIter<ScopeEntryWithVis> {
        self
    }
}

pub fn check_if_visible(
    db: &dyn SourceDatabase,
    scope_entry: ScopeEntry,
    context: InFile<impl Into<SyntaxElement>>,
) -> ScopeEntryWithVis {
    let invis_reason = is_visible_in_context(db, &scope_entry, context);
    ScopeEntryWithVis { scope_entry, invis_reason }
}

pub fn is_visible_in_context(
    db: &dyn SourceDatabase,
    scope_entry: &ScopeEntry,
    context: InFile<impl Into<SyntaxElement>>,
) -> Option<ItemInvisibleReason> {
    use syntax::SyntaxKind::*;

    let (context_file_id, context) = context.map(|it| it.into()).unpack();

    // inside msl everything is visible
    if context.is_msl_context() {
        return None;
    }

    // if inside MvAttrItem like abort_code=
    if context.ancestor_strict::<ast::AttrItem>().is_some() {
        return None;
    }

    let Some(InFile {
        file_id: item_file_id,
        value: item,
    }) = scope_entry.node_loc.to_ast::<ast::NamedElement>(db)
    else {
        return Some(ItemInvisibleReason::Unknown);
    };
    let item_kind = item.syntax().kind();
    let item_ns = scope_entry.ns;
    let opt_visible_item = ast::AnyHasVisibility::cast(item.syntax().clone());

    let context_loc = SyntaxLoc::from_file_syntax_node(&context.loc_node().in_file(context_file_id));
    let context_item_scope = hir_db::item_scope(db, context_loc);

    let context_opt_path = context.as_node().and_then(|it| it.cast::<ast::Path>());
    if let Some(path) = &context_opt_path
        && path.root_parent_of_type::<ast::UseSpeck>().is_some()
    {
        // those are always public in use specks
        if matches!(item_kind, MODULE | STRUCT | ENUM) {
            return None;
        }

        // items needs to be non-private to be visible, no other rules apply in use specks
        if let Some(visible_item) = opt_visible_item.clone() {
            if visible_item.vis() != Vis::Private {
                return None;
            }
        }

        // msl-only items are available from imports
        if item.syntax().is_msl_only_item() {
            return None;
        }

        // consts are importable in tests
        if context_item_scope.is_test() && item_ns == Ns::NAME {
            return None;
        }
    }

    let item_module = item.syntax().containing_module();
    // 0x0::builtins module functions and consts are always visible
    if matches!(item.syntax().kind(), FUN | CONST)
        && item_module.clone().is_some_and(|m| m.is_builtins())
    {
        return None;
    }

    let item_scope = {
        let item_loc = item.clone().in_file(item_file_id).loc();
        let mut item_scope = hir_db::item_scope(db, item_loc);
        if let Some(adjustment) = scope_entry.scope_adjustment {
            item_scope = item_scope.shrink_scope(adjustment);
        }
        item_scope
    };

    // i.e. #[test_only] items in non-test-only scope
    if item_scope != ItemScope::Main {
        // cannot be used everywhere, need to check for scope compatibility
        if item_scope != context_item_scope {
            return Some(ItemInvisibleReason::WrongItemScope { item_scope });
        }
    }

    // we're in non-msl scope at this point, msl only items aren't accessible
    if item.syntax().is_msl_only_item() {
        return Some(ItemInvisibleReason::WrongItemScope {
            item_scope: ItemScope::Verify,
        });
    }

    // local methods, Self::method - everything is visible
    let context_module = context.containing_module();
    if item_module.is_some() && context_module.is_some() && item_module == context_module {
        return None;
    }

    // local items in script
    if let Some(context_script) = context.containing_script() {
        if item
            .syntax()
            .containing_script()
            .is_some_and(|it| context_script == it)
        {
            return None;
        }
    }

    // item is type, check whether it's allowed in the context
    if TYPES_N_ENUMS.contains(item_ns) {
        let opt_path_parent = context_opt_path
            .map(|path| path.root_path())
            .and_then(|it| it.syntax().parent());
        if let Some(path_parent) = opt_path_parent {
            if path_parent.kind() == PATH_TYPE {
                return None;
            }
        }
    }
    let vis = opt_visible_item
        .map(|visible_item| visible_item.vis())
        .unwrap_or(Vis::Public);
    match vis {
        Vis::Private => Some(ItemInvisibleReason::Private { vis }),
        Vis::Public => None,
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
                            return None;
                        }
                    }
                }
                Some(ItemInvisibleReason::Private { vis })
            }
            VisLevel::Package => {
                // check for the same source root
                if db.file_package_id(context_file_id) == db.file_package_id(item_file_id) {
                    None
                } else {
                    Some(ItemInvisibleReason::Private { vis })
                }
            }
        },
    }
}
