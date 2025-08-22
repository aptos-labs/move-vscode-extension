// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

pub mod any_field_ext;
pub mod callable;
pub mod item;
pub mod item_spec;

use crate::nameres::address::{Address, NamedAddr, ValueAddr};
use syntax::ast;

pub trait ModuleLangExt {
    fn address(&self) -> Option<Address>;
    fn is_builtins(&self) -> bool;
}

impl ModuleLangExt for ast::Module {
    fn address(&self) -> Option<Address> {
        let address_ref = self.self_or_parent_address_ref()?;
        match address_ref {
            ast::AddressRef::NamedAddress(named) => {
                Some(Address::Named(NamedAddr::new(named.ident_token().to_string())))
            }
            ast::AddressRef::ValueAddress(value) => {
                Some(Address::Value(ValueAddr::new(value.address_text())))
            }
        }
    }
    fn is_builtins(&self) -> bool {
        let name = self.name().map(|n| n.to_string());
        if name.is_some_and(|it| it == "builtins") {
            let address = self.address();
            return address.is_some() && address.unwrap().is_0x0();
        }
        false
    }
}
