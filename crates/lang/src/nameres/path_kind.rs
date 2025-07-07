// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::nameres::address::{Address, NamedAddr, ValueAddr, resolve_named_address};
use crate::nameres::namespaces::{
    ALL_NS, ENUMS_N_MODULES, IMPORTABLE_NS, MODULES, NAMES, NAMES_N_FUNCTIONS_N_VARIANTS, Ns, NsSet,
    SCHEMAS, TYPES_N_ENUMS, TYPES_N_ENUMS_N_ENUM_VARIANTS, TYPES_N_ENUMS_N_MODULES,
    TYPES_N_ENUMS_N_NAMES,
};
use enumset::enum_set;
use std::fmt;
use std::fmt::Formatter;
use syntax::SyntaxKind::*;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::ast::node_ext::syntax_node::{SyntaxNodeExt, SyntaxTokenExt};
use syntax::{AstNode, SyntaxNode, T, ast};

#[derive(Clone, PartialEq, Eq)]
pub enum PathKind {
    // aptos_std:: where aptos_std is a existing named address in a project
    NamedAddress(NamedAddr),
    // 0x1::
    ValueAddress(ValueAddr),
    // aptos_std:: where aptos_std is a existing named address in a project
    NamedAddressOrUnqualifiedPath {
        address: NamedAddr,
        ns: NsSet,
    },
    // MyStruct { foo }
    //            ^^^
    FieldShorthand {
        struct_field: ast::StructLitField,
    },
    // foo
    Unqualified {
        ns: NsSet,
    },

    // any multi element path
    Qualified {
        qualifier: ast::Path,
        ns: NsSet,
        kind: QualifiedKind,
    },
}

impl fmt::Debug for PathKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PathKind::NamedAddress(_) => f.debug_struct("NamedAddress").finish(),
            PathKind::ValueAddress(_) => f.debug_struct("ValueAddress").finish(),
            PathKind::NamedAddressOrUnqualifiedPath { ns, .. } => f
                .debug_struct("NamedAddressOrUnqualifiedPath")
                .field("ns", &ns)
                .finish(),
            PathKind::FieldShorthand { .. } => f.debug_struct("FieldShorthand").finish(),
            PathKind::Unqualified { ns } => f.debug_struct("Unqualified").field("ns", &ns).finish(),
            PathKind::Qualified { qualifier, ns, kind } => f
                .debug_struct("Qualified")
                .field("kind", &kind)
                .field("qualifier", &qualifier.syntax().text())
                .field("ns", &ns)
                .finish(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QualifiedKind {
    // `0x1:foo`
    Module { address: Address },
    // `aptos_framework::foo` (where aptos_framework is known named address, but it can still be a module)
    ModuleOrItem { address: Address },
    // bar in foo::bar, where foo is not a named address
    ModuleItemOrEnumVariant,
    // bar in `0x1::foo::bar` or `aptos_std::foo::bar` (where aptos_std is known named address)
    FQModuleItem,
    // use 0x1::m::{item1};
    //               ^
    UseGroupItem,
}

/// can return None on deeply invalid trees
pub fn path_kind(
    qualifier: Option<ast::Path>,
    path: ast::Path,
    is_completion: bool,
) -> Option<PathKind> {
    if let Some(use_group) = path.syntax().ancestor_strict::<ast::UseGroup>() {
        // use 0x1::m::{item}
        //                ^
        let parent_use_speck = use_group.syntax().parent_of_type::<ast::UseSpeck>()?;
        let use_group_qualifier = parent_use_speck.path()?;
        return Some(PathKind::Qualified {
            qualifier: use_group_qualifier,
            ns: IMPORTABLE_NS | MODULES,
            kind: QualifiedKind::UseGroupItem,
        });
    }

    let path_parent = path.syntax().parent()?;

    // [0x1::foo]::bar
    //     ^ qualifier
    let has_trailing_colon_colon = path
        .ident_token()
        .and_then(|it| it.next_token())
        .is_some_and(|token| token.is(T![::]));

    let ns = path_namespaces(qualifier.as_ref(), path_parent, is_completion);

    // one-element path
    if qualifier.is_none() {
        // if path_address exists, it means it has to be a value address
        if let Some(path_address) = path.path_address() {
            return Some(PathKind::ValueAddress(ValueAddr::new(
                path_address.value_address().address_text(),
            )));
        }

        let ref_name = path.reference_name().expect("as `path_address` is None");

        // check whether it's a first element in use stmt, i.e. use [std]::module;
        if let Some(use_speck) = path.root_parent_of_type::<ast::UseSpeck>() {
            if use_speck.syntax().parent_is::<ast::UseStmt>() {
                return Some(PathKind::NamedAddress(NamedAddr::new(ref_name)));
            }
        }

        // outside use stmt
        // check whether there's a '::' after it, then try for a named address
        if has_trailing_colon_colon {
            if path
                .root_parent_kind()
                .is_some_and(|it| matches!(it, FRIEND | MODULE_SPEC))
            {
                // friend addr::module;
                //        ^ (unknown named address)
                return Some(PathKind::NamedAddress(NamedAddr::new(ref_name)));
            }

            // todo: add resolve back when we have named addresses parsed, it improves unresolved reference a lot
            // if resolve_named_address(&ref_name).is_some() {
            return Some(PathKind::NamedAddressOrUnqualifiedPath {
                address: NamedAddr::new(ref_name),
                ns,
            });
            // }
        }

        if let Some(path_name_ref) = path.segment().and_then(|it| it.name_ref()) {
            if let Some(ast::StructLitFieldKind::Shorthand { struct_field, .. }) =
                path_name_ref.try_into_struct_lit_field()
            {
                return Some(PathKind::FieldShorthand { struct_field });
            }
        }

        return Some(PathKind::Unqualified { ns });
    }

    let qualifier = qualifier.unwrap();
    let qualifier_of_qualifier = qualifier.qualifier();

    // two-element paths
    if qualifier_of_qualifier.is_none() {
        let qualifier_path_address = qualifier.path_address();
        if let Some(qualifier_path_address) = qualifier_path_address {
            let value_address = Address::Value(ValueAddr::new(
                qualifier_path_address.value_address().address_text(),
            ));
            return Some(PathKind::Qualified {
                qualifier,
                ns: MODULES,
                kind: QualifiedKind::Module { address: value_address },
            });
        }

        let Some(qualifier_ref_name) = qualifier.reference_name() else {
            // should be either address or reference name, cannot be none
            return None;
        };

        // use std::[main] | friend std::[main]; | spec std::[main] {}
        if path
            .root_parent_kind()
            .is_some_and(|it| matches!(it, USE_SPECK | FRIEND | MODULE_SPEC))
        {
            return Some(PathKind::Qualified {
                qualifier,
                ns: MODULES,
                kind: QualifiedKind::Module {
                    address: Address::Named(NamedAddr::new(qualifier_ref_name)),
                },
            });
        }

        let named_address = resolve_named_address(&qualifier_ref_name);
        if let Some(_) = named_address {
            // known named address, can be module path, or module item path too
            return Some(PathKind::Qualified {
                qualifier,
                ns,
                kind: QualifiedKind::ModuleOrItem {
                    address: Address::Named(NamedAddr::new(qualifier_ref_name)),
                },
            });
        }

        // item1::[item2]::
        // ^ qualifier_ref_name
        //            ^ path
        if has_trailing_colon_colon {
            return Some(PathKind::Qualified {
                qualifier,
                ns,
                kind: QualifiedKind::ModuleOrItem {
                    address: Address::Named(NamedAddr::new(qualifier_ref_name)),
                },
            });
        }

        // remove MODULE if it's added, as it cannot be a MODULE
        let mut ns = ns;
        ns.remove(Ns::MODULE);

        // module::[name]
        return Some(PathKind::Qualified {
            qualifier,
            ns,
            kind: QualifiedKind::ModuleItemOrEnumVariant,
        });
    }

    if path.root_parent_of_type::<ast::UseSpeck>().is_some() {
        // MODULES are for `use 0x1::m::Self;`
        return Some(PathKind::Qualified {
            qualifier,
            ns: ns | MODULES,
            kind: QualifiedKind::FQModuleItem,
        });
    }

    // three-element path
    Some(PathKind::Qualified {
        qualifier,
        // remove MODULE if it's added, as it cannot be a MODULE
        ns: ns - Ns::MODULE,
        kind: QualifiedKind::FQModuleItem,
    })
}

fn path_namespaces(
    qualifier: Option<&ast::Path>,
    path_parent: SyntaxNode,
    is_completion: bool,
) -> NsSet {
    use syntax::SyntaxKind::*;

    match path_parent.kind() {
        // mod::foo::bar
        //      ^
        PATH if qualifier.is_some() => enum_set!(Ns::MODULE | Ns::ENUM),
        // foo::bar
        //  ^
        PATH => {
            ENUMS_N_MODULES
            // // if we're inside PathType, then ENUM::ENUM_VARIANT cannot be used
            // if parent.parent().is_kind(PATH_TYPE) {
            //     MODULES
            // } else {
            //     TYPES_N_MODULES
            // }
        }
        // use 0x1::foo::bar; | use 0x1::foo::{bar, baz}
        //               ^                     ^
        USE_SPECK => IMPORTABLE_NS,

        PATH_TYPE if path_parent.parent_is::<ast::IsExpr>() => TYPES_N_ENUMS_N_ENUM_VARIANTS,

        // a: bar
        //     ^
        PATH_TYPE if qualifier.is_none() => {
            if is_completion {
                TYPES_N_ENUMS_N_MODULES
            } else {
                TYPES_N_ENUMS
            }
        }
        // a: foo::bar
        //         ^
        PATH_TYPE if qualifier.is_some() => {
            if is_completion {
                TYPES_N_ENUMS_N_MODULES
            } else {
                TYPES_N_ENUMS
            }
        }

        CALL_EXPR => NAMES_N_FUNCTIONS_N_VARIANTS,

        PATH_EXPR if path_parent.has_ancestor_or_self::<ast::AttrItem>() => ALL_NS,

        // TYPE | ENUM for resource indexing, NAME for vector indexing
        PATH_EXPR if path_parent.parent_is::<ast::IndexExpr>() => TYPES_N_ENUMS_N_NAMES,

        // can be anything in completion
        PATH_EXPR => {
            if is_completion {
                ALL_NS
            } else {
                NAMES_N_FUNCTIONS_N_VARIANTS
                // NAMES_N_VARIANTS
            }
        }

        SCHEMA_LIT => SCHEMAS,

        STRUCT_LIT | STRUCT_PAT | TUPLE_STRUCT_PAT | PATH_PAT => TYPES_N_ENUMS_N_ENUM_VARIANTS,

        // todo:

        //     parent is MvAccessSpecifier -> TYPES_N_ENUMS
        //     parent is MvAddressSpecifierArg -> NAMES
        //     parent is MvAddressSpecifierCallParam -> NAMES
        FRIEND => MODULES,
        MODULE_SPEC => MODULES,

        //
        // // should not be used for attr items
        //     parent is MvAttrItem -> NONE

        // todo: error if not handled
        _ => NAMES,
    }
}

// fn get_named_address(_address_name: &str) -> Option<NamedAddr> {
//     // todo: check whether `address_name` is a declared named address
//     // todo: fetch from db
//     None
// }
