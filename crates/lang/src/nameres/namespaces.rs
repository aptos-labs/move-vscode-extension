// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use enumset::{EnumSet, EnumSetType, enum_set};
use std::fmt;
use std::fmt::Formatter;
use syntax::{AstNode, ast};

#[allow(non_camel_case_types)]
#[derive(EnumSetType, Debug, Hash)]
pub enum Ns {
    NAME,
    FUNCTION,
    TYPE,
    TUPLE_STRUCT,
    ENUM,
    ENUM_VARIANT,
    SCHEMA,
    MODULE,
}

impl fmt::Display for Ns {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{:?}", self))
    }
}

pub type NsSet = EnumSet<Ns>;

pub const NAMES: NsSet = enum_set!(Ns::NAME);
pub const FUNCTIONS: NsSet = enum_set!(Ns::FUNCTION);
pub const ENUM_VARIANTS: NsSet = enum_set!(Ns::ENUM_VARIANT);
pub const SCHEMAS: NsSet = enum_set!(Ns::SCHEMA);
pub const MODULES: NsSet = enum_set!(Ns::MODULE);

pub const ENUMS_N_MODULES: NsSet = enum_set!(Ns::ENUM | Ns::MODULE);

// variables | enum variants
pub const VALUE_NS: NsSet = enum_set!(Ns::NAME | Ns::FUNCTION | Ns::ENUM_VARIANT);
pub const CONTAINER_TYPE_NS: NsSet =
    enum_set!(Ns::TYPE | Ns::TUPLE_STRUCT | Ns::ENUM | Ns::ENUM_VARIANT);
pub const CALLABLE_NS: NsSet = enum_set!(Ns::NAME | Ns::FUNCTION | Ns::TUPLE_STRUCT | Ns::ENUM_VARIANT);
// vector | resource types
pub const INDEXABLE_NS: NsSet = enum_set!(Ns::TYPE | Ns::ENUM | Ns::TUPLE_STRUCT | Ns::NAME);

pub const IMPORTABLE_NS: NsSet =
    enum_set!(Ns::NAME | Ns::FUNCTION | Ns::TYPE | Ns::TUPLE_STRUCT | Ns::SCHEMA | Ns::ENUM);
pub const ITEM_TYPE_NS: NsSet = enum_set!(Ns::TYPE | Ns::TUPLE_STRUCT | Ns::ENUM);
// pub const ITEM_TYPE_NS_N_MODULES: NsSet = ITEM_TYPE_NS | enum_set!();

pub const NONE: NsSet = enum_set!();
pub const ALL_NS: NsSet = enum_set!(
    Ns::NAME
        | Ns::FUNCTION
        | Ns::TYPE
        | Ns::TUPLE_STRUCT
        | Ns::ENUM
        | Ns::ENUM_VARIANT
        | Ns::SCHEMA
        | Ns::MODULE
);

pub trait NsSetExt {
    fn contains_any_of(&self, other: NsSet) -> bool;
}

impl NsSetExt for NsSet {
    fn contains_any_of(&self, other: NsSet) -> bool {
        !self.is_disjoint(other)
    }
}

pub(crate) fn named_item_ns(named_element: &ast::NamedElement) -> Ns {
    use syntax::SyntaxKind::*;
    let named_item_kind = named_element.syntax().kind();
    match named_item_kind {
        MODULE => Ns::MODULE,
        SPEC_FUN | FUN | SPEC_INLINE_FUN => Ns::FUNCTION,
        TYPE_PARAM => Ns::TYPE,
        STRUCT => {
            let struct_ = named_element.clone().struct_().expect("is STRUCT");
            if struct_.is_tuple_struct() {
                Ns::TUPLE_STRUCT
            } else {
                Ns::TYPE
            }
        }
        // TYPE_PARAM | STRUCT => Ns::TYPE,
        ENUM => Ns::ENUM,
        VARIANT => Ns::ENUM_VARIANT,
        IDENT_PAT | TUPLE_FIELD | NAMED_FIELD | CONST | GLOBAL_VARIABLE_DECL => Ns::NAME,
        SCHEMA => Ns::SCHEMA,
        SCHEMA_FIELD => Ns::NAME,
        _ => unreachable!(
            "named nodes should be exhaustive, unhandled {:?}",
            named_item_kind
        ),
    }
}
