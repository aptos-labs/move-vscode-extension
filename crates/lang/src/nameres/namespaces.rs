use enumset::{EnumSet, EnumSetType, enum_set};
use std::fmt;
use std::fmt::Formatter;

#[allow(non_camel_case_types)]
#[derive(EnumSetType, Debug, Hash)]
pub enum Ns {
    NAME,
    FUNCTION,
    TYPE,
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
pub const NAMES_N_VARIANTS: NsSet = enum_set!(Ns::NAME | Ns::ENUM_VARIANT);
pub const NAMES_N_FUNCTIONS_N_VARIANTS: NsSet = enum_set!(Ns::NAME | Ns::FUNCTION | Ns::ENUM_VARIANT);
pub const TYPES: NsSet = enum_set!(Ns::TYPE);
pub const ENUMS: NsSet = enum_set!(Ns::ENUM);
pub const ENUM_VARIANTS: NsSet = enum_set!(Ns::ENUM_VARIANT);
pub const SCHEMAS: NsSet = enum_set!(Ns::SCHEMA);
pub const MODULES: NsSet = enum_set!(Ns::MODULE);

pub const ENUMS_N_MODULES: NsSet = enum_set!(Ns::ENUM | Ns::MODULE);
pub const TYPES_N_MODULES: NsSet = enum_set!(Ns::TYPE | Ns::MODULE);
pub const TYPES_N_ENUMS_N_MODULES: NsSet = enum_set!(Ns::TYPE | Ns::ENUM | Ns::MODULE);
pub const TYPES_N_ENUMS_N_ENUM_VARIANTS: NsSet = enum_set!(Ns::TYPE | Ns::ENUM | Ns::ENUM_VARIANT);
pub const TYPES_N_ENUMS_N_ENUM_VARIANTS_N_MODULES: NsSet =
    enum_set!(Ns::TYPE | Ns::ENUM | Ns::ENUM_VARIANT | Ns::MODULE);
pub const TYPES_N_ENUMS: NsSet = enum_set!(Ns::TYPE | Ns::ENUM);
pub const TYPES_N_NAMES: NsSet = enum_set!(Ns::TYPE | Ns::NAME);
pub const TYPES_N_ENUMS_N_NAMES: NsSet = enum_set!(Ns::TYPE | Ns::ENUM | Ns::NAME);

pub const NONE: NsSet = enum_set!();
pub const IMPORTABLE_NS: NsSet = enum_set!(Ns::NAME | Ns::FUNCTION | Ns::TYPE | Ns::SCHEMA | Ns::ENUM);
pub const ALL_NS: NsSet = enum_set!(
    Ns::NAME | Ns::FUNCTION | Ns::TYPE | Ns::ENUM | Ns::ENUM_VARIANT | Ns::SCHEMA | Ns::MODULE
);

pub trait NsSetExt {
    fn contains_any_of(&self, other: NsSet) -> bool;
}

impl NsSetExt for NsSet {
    fn contains_any_of(&self, other: NsSet) -> bool {
        !self.is_disjoint(other)
    }
}

pub(crate) fn named_item_ns(named_item_kind: syntax::SyntaxKind) -> Ns {
    use syntax::SyntaxKind::*;
    match named_item_kind {
        MODULE => Ns::MODULE,
        SPEC_FUN | FUN | SPEC_INLINE_FUN => Ns::FUNCTION,
        TYPE_PARAM | STRUCT => Ns::TYPE,
        ENUM => Ns::ENUM,
        VARIANT => Ns::ENUM_VARIANT,
        IDENT_PAT | NAMED_FIELD | CONST | GLOBAL_VARIABLE_DECL => Ns::NAME,
        SCHEMA => Ns::SCHEMA,
        SCHEMA_FIELD => Ns::NAME,
        _ => unreachable!(
            "named nodes should be exhaustive, unhandled {:?}",
            named_item_kind
        ),
    }
}
