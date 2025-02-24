use enumset::{enum_set, EnumSet, EnumSetType};

#[derive(EnumSetType, Debug)]
pub enum Namespace {
    TYPE,
    NAME,
    MODULE,
    SCHEMA,
}

pub type NsSet = EnumSet<Namespace>;

pub const NAMES: NsSet = enum_set!(Namespace::NAME);
pub const TYPES: NsSet = enum_set!(Namespace::TYPE);
pub const MODULES: NsSet = enum_set!(Namespace::MODULE);
pub const SCHEMAS: NsSet = enum_set!(Namespace::SCHEMA);

pub const TYPES_N_MODULES: NsSet = enum_set!(Namespace::TYPE | Namespace::MODULE);
pub const TYPES_N_NAMES: NsSet = enum_set!(Namespace::TYPE | Namespace::NAME);

pub const NONE: NsSet = enum_set!();
pub const MODULE_ITEMS: NsSet = enum_set!(Namespace::NAME | Namespace::TYPE | Namespace::SCHEMA);
pub const ALL: NsSet =
    enum_set!(Namespace::NAME | Namespace::TYPE | Namespace::SCHEMA | Namespace::MODULE);

pub trait NsSetExt {
    fn contains_any_of(&self, other: NsSet) -> bool;
}

impl NsSetExt for NsSet {
    fn contains_any_of(&self, other: NsSet) -> bool {
        !self.is_disjoint(other)
    }
}
