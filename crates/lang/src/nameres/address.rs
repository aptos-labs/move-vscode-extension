// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::hir_db;
use crate::hir_db::APTOS_FRAMEWORK_ADDRESSES;
use base_db::SourceDatabase;
use std::fmt;
use std::fmt::Formatter;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Address {
    Named(NamedAddr),
    Value(ValueAddr),
}

impl Address {
    pub fn named(name: &str) -> Self {
        Address::Named(NamedAddr::new(name.to_string()))
    }

    pub fn value(value: &str) -> Self {
        Address::Value(ValueAddr::new(value.to_string()))
    }

    pub fn resolve_to_numeric_address(&self, db: &dyn SourceDatabase) -> Option<NumericAddress> {
        match self {
            Address::Named(named_addr) => resolve_named_address(db, named_addr.name.as_str()),
            Address::Value(value_addr) => Some(value_addr.numeric_address.clone()),
        }
    }

    pub fn is_0x0(&self) -> bool {
        match self {
            Address::Value(value_addr) => value_addr.numeric_address.short() == "0x0",
            _ => false,
        }
    }

    pub fn is_0x1(&self) -> bool {
        match self {
            Address::Value(value_addr) => value_addr.numeric_address.short() == "0x1",
            _ => false,
        }
    }

    pub fn identifier_text(&self) -> String {
        match self {
            Address::Named(named_addr) => named_addr.name.clone(),
            Address::Value(value_addr) => value_addr.numeric_address.value.clone(),
        }
    }

    pub fn equals_to(
        &self,
        db: &dyn SourceDatabase,
        candidate_address: &Address,
        is_completion: bool,
    ) -> bool {
        if self == candidate_address {
            return true;
        }

        let self_val = self.resolve_to_numeric_address(db);
        let candidate_val = candidate_address.resolve_to_numeric_address(db);

        let same_values = match (self_val, candidate_val) {
            (Some(self_val), Some(cand_val)) => {
                // if any of the addresses is '_', then only equal if names are the same (checked before)
                !self_val.is_underscore()
                    && !cand_val.is_underscore()
                    && self_val.short() == cand_val.short()
            }
            _ => false,
        };

        if same_values && is_completion {
            // compare named addresses by name in case of the same values for the completion
            match (self, candidate_address) {
                (Address::Named(left_named), Address::Named(right_named)) => {
                    return left_named == right_named;
                }
                _ => {}
            }
        }

        same_values
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Address::Named(named) => f.debug_tuple("Address.Named").field(&named.name).finish(),
            Address::Value(value) => f
                .debug_tuple("Address.Value")
                .field(&value.numeric_address.original())
                .finish(),
        }
    }
}

#[salsa_macros::interned(debug)]
pub struct AddressInput {
    pub data: Address,
}

pub fn resolve_named_address(db: &dyn SourceDatabase, name: &str) -> Option<NumericAddress> {
    if APTOS_FRAMEWORK_ADDRESSES.contains(&name) {
        return Some(NumericAddress { value: "0x1".to_string() });
    }
    let named_addresses = hir_db::named_addresses(db);
    let address_value = named_addresses.get(name)?;
    // let (_, address_value) = named_addresses.iter().find(|(it, _)| it == name)?;
    Some(NumericAddress {
        value: address_value.to_owned(),
    })
    // if named_addresses.iter().any(|it| it.0 == name) {
    //     Some(NumericAddress { value: "_".to_string() })
    // } else {
    //     None
    // }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NumericAddress {
    value: String,
}

impl NumericAddress {
    pub fn is_underscore(&self) -> bool {
        self.value == "_"
    }
    pub fn original(&self) -> String {
        self.value.to_string()
    }
    pub fn short(&self) -> String {
        let text = self.value.as_str();
        if !text.starts_with("0") {
            return text.to_string();
        }
        let trimmed = if text.starts_with("0x") {
            &text[2..]
        } else {
            &text[1..]
        };
        let mut trimmed_address = trimmed.trim_start_matches("0");
        if trimmed_address.is_empty() {
            trimmed_address = "0";
        }
        format!("0x{}", trimmed_address)
    }

    pub fn normalized(&self) -> String {
        self.short()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamedAddr {
    name: String,
}

impl NamedAddr {
    pub fn new(name: String) -> Self {
        NamedAddr { name }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValueAddr {
    numeric_address: NumericAddress,
}

impl ValueAddr {
    pub fn new(value: String) -> Self {
        ValueAddr {
            numeric_address: NumericAddress { value },
        }
    }
}
