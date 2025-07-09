// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

pub mod any_field_ext;
pub mod callable;
pub mod has_item_list;
pub mod item;
pub mod item_spec;

use crate::nameres::address::{Address, NamedAddr, ValueAddr};
use syntax::ast;

pub trait ModuleLangExt {
    fn address(&self) -> Option<Address>;
    fn address_equals_to(&self, address: Address, is_completion: bool) -> bool;
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

    fn address_equals_to(&self, candidate_address: Address, is_completion: bool) -> bool {
        let self_address = self.address();
        let Some(self_address) = self_address else {
            return false;
        };
        if self_address == candidate_address {
            return true;
        }

        let self_numeric = self_address.clone().resolve_to_numeric_address();
        let candidate_numeric = candidate_address.clone().resolve_to_numeric_address();
        let same_values = match (self_numeric, candidate_numeric) {
            (Some(left), Some(right)) => left.short() == right.short(),
            _ => false,
        };

        if same_values && is_completion {
            // compare named addresses by name in case of the same values for the completion
            match (self_address, candidate_address) {
                (Address::Named(left_named), Address::Named(right_named)) => {
                    return left_named == right_named;
                }
                _ => {}
            }
        }

        same_values
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
