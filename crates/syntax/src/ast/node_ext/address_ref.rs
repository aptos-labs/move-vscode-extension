// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::AddressRef {
    pub fn address_text(&self) -> String {
        match self {
            ast::AddressRef::NamedAddress(named_address) => named_address.to_string(),
            ast::AddressRef::ValueAddress(value_address) => value_address.address_text(),
        }
    }
}

impl ast::ValueAddress {
    pub fn address_text(&self) -> String {
        self.int_number_token().text().to_string()
    }
}
