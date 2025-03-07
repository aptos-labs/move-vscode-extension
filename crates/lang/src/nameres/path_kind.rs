use crate::nameres::address::{Address, NamedAddress, ValueAddress};
use crate::nameres::namespaces::{
    NsSet, ALL_NS, IMPORTABLE_NS, MODULES, NAMES, NONE, TYPES, TYPES_N_MODULES, TYPES_N_NAMES,
};
use parser::T;
use syntax::ast::node_ext::syntax_node::{OptionSyntaxNodeExt, SyntaxNodeExt};
use syntax::{ast, AstNode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathKind {
    // aptos_std:: where aptos_std is a existing named address in a project
    NamedAddress(NamedAddress),
    // 0x1::
    ValueAddress(ValueAddress),
    // // aptos_std:: where aptos_std is a existing named address in a project
    NamedAddressOrUnqualifiedPath {
        address: NamedAddress,
        ns: NsSet,
    },
    // foo
    Unqualified {
        ns: NsSet,
    },

    // any multi element path
    Qualified {
        path: ast::Path,
        qualifier: ast::Path,
        ns: NsSet,
        kind: QualifiedKind,
    },
}

impl PathKind {
    pub fn is_unqualified(&self) -> bool {
        matches!(self, PathKind::Unqualified { .. })
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
pub fn path_kind(path: ast::Path, is_completion: bool) -> Option<PathKind> {
    if let Some(use_group) = path.syntax().ancestor_strict::<ast::UseGroup>() {
        // use 0x1::m::{item}
        //                ^
        let parent_use_speck = use_group.syntax().parent_of_type::<ast::UseSpeck>()?;
        let use_group_qualifier = parent_use_speck.path()?;
        return Some(PathKind::Qualified {
            path,
            qualifier: use_group_qualifier,
            ns: IMPORTABLE_NS,
            kind: QualifiedKind::UseGroupItem,
        });
    }

    // [0x1::foo]::bar
    //     ^ qualifier
    let qualifier = path.qualifier();
    let ns = path_namespaces(path.clone(), is_completion);

    // one-element path
    if qualifier.is_none() {
        let path_address = path.path_address();
        // if path_address exists, it means it has to be a value address
        if let Some(path_address) = path.path_address() {
            return Some(PathKind::ValueAddress(ValueAddress::new(
                path_address.value_address().address_text(),
            )));
        }
        if path_address.is_some() {}

        let name_ref = path.name_ref().expect("should be Some if path_address is None");
        let ref_name = name_ref.ident_token().text().to_string();

        // check whether it's a first element in use stmt, i.e. use [std]::module;
        if let Some(use_speck) = path.use_speck() {
            if use_speck.syntax().parent_of_type::<ast::UseItem>().is_some() {
                if let Some(existing_named_address) = get_named_address(&ref_name) {
                    return Some(PathKind::NamedAddress(existing_named_address));
                }
                return Some(PathKind::NamedAddress(NamedAddress::new(ref_name)));
            }
        }

        // outside use stmt
        // check whether there's a '::' after it, then try for a named address
        if let Some(next_sibling) = path.syntax().next_sibling_or_token_no_trivia() {
            if next_sibling.kind() == T![::] {
                if let Some(named_address) = get_named_address(&ref_name) {
                    return Some(PathKind::NamedAddressOrUnqualifiedPath {
                        address: named_address,
                        ns,
                    });
                }
            }
        }

        return Some(PathKind::Unqualified { ns });
    }

    let qualifier = qualifier.unwrap();
    let qualifier_of_qualifier = qualifier.qualifier();

    // two-element paths
    if qualifier_of_qualifier.is_none() {
        let qualifier_path_address = qualifier.path_address();
        let qualifier_ref_name = qualifier.name_ref().map(|it| it.ident_token().text().to_string());

        match (qualifier_path_address, qualifier_ref_name) {
            // 0x1::[bar]
            (Some(qualifier_path_address), _) => {
                let value_address = Address::Value(ValueAddress::new(
                    qualifier_path_address.value_address().address_text(),
                ));
                return Some(PathKind::Qualified {
                    path,
                    qualifier,
                    ns,
                    kind: QualifiedKind::Module { address: value_address },
                });
            }
            // aptos_framework::[bar]
            (_, Some(qualifier_ref_name)) /*if aptos_project.is_some()*/ => {
                let named_address = get_named_address(&qualifier_ref_name);
                // use std::[main]
                if path.use_speck().is_some() {
                    let address = named_address.unwrap_or(NamedAddress::new(qualifier_ref_name));
                    return Some(PathKind::Qualified {
                        path,
                        qualifier,
                        ns,
                        kind: QualifiedKind::Module { address: Address::Named(address) }
                    });
                }
                if let Some(named_address) = named_address {
                    // known named address, can be module path, or module item path too
                    return Some(PathKind::Qualified {
                        path,
                        qualifier,
                        ns,
                        kind: QualifiedKind::ModuleOrItem { address: Address::Named(named_address) }
                    });
                }
            }
            _ => ()
        }

        // module::[name]
        return Some(PathKind::Qualified {
            path,
            qualifier,
            ns,
            kind: QualifiedKind::ModuleItemOrEnumVariant,
        });
    }

    // three-element path
    Some(PathKind::Qualified {
        path,
        qualifier,
        ns,
        kind: QualifiedKind::FQModuleItem,
    })
}

fn path_namespaces(path: ast::Path, is_completion: bool) -> NsSet {
    use syntax::SyntaxKind::*;

    let qualifier = path.qualifier();
    let Some(parent) = path.syntax().parent() else {
        return NONE;
    };

    match parent.kind() {
        // mod::foo::bar
        //      ^
        PATH if qualifier.is_some() => TYPES,
        // foo::bar
        //  ^
        PATH => {
            // if we're inside PathType, then ENUM::ENUM_VARIANT cannot be used
            if parent.parent().is_kind(PATH_TYPE) {
                MODULES
            } else {
                TYPES_N_MODULES
            }
        },
        // use 0x1::foo::bar; | use 0x1::foo::{bar, baz}
        //               ^                     ^
        USE_SPECK => IMPORTABLE_NS,
        // a: foo::bar
        //         ^
        PATH_TYPE if qualifier.is_some() => TYPES,
        // a: bar
        //     ^
        PATH_TYPE => {
            if is_completion {
                TYPES_N_MODULES
            } else {
                TYPES
            }
        }
        CALL_EXPR => NAMES,

        // todo: change into AttrItemInitializer
        PATH_EXPR if path.syntax().has_ancestor_strict::<ast::AttrItem>() => ALL_NS,
        // TYPE | ENUM for resource indexing, NAME for vector indexing
        PATH_EXPR if parent.parent().is_kind(INDEX_EXPR) => {
            TYPES_N_NAMES
        }

        // can be anything in completion
        PATH_EXPR => {
            if is_completion {
                ALL_NS
            } else {
                NAMES
            }
        }

        // todo:
        // parent is MvSchemaLit
        //     || parent is MvSchemaRef -> SCHEMAS

        // todo:
        STRUCT_LIT | STRUCT_PAT | TUPLE_STRUCT_PAT /*| CONST_PAT*/ => TYPES,

        // todo:

        //     parent is MvAccessSpecifier -> TYPES_N_ENUMS
        //     parent is MvAddressSpecifierArg -> NAMES
        //     parent is MvAddressSpecifierCallParam -> NAMES

        FRIEND => MODULES,

        //     parent is MvModuleSpec -> MODULES
        //
        // // should not be used for attr items
        //     parent is MvAttrItem -> NONE

        // todo: error if not handled
        _ => NAMES,
    }
}

fn get_named_address(_address_name: &str) -> Option<NamedAddress> {
    // todo: check whether `address_name` is a declared named address
    // todo: fetch from db
    None
}
