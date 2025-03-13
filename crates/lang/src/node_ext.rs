pub mod has_item_list;

use crate::nameres::address::{Address, NamedAddr, ValueAddr};
use crate::{AsName, Name};
use syntax::ast;

pub trait PathLangExt {
    fn name_ref_name(&self) -> Option<Name>;
}

impl PathLangExt for ast::Path {
    fn name_ref_name(&self) -> Option<Name> {
        self.name_ref().map(|name_ref| name_ref.as_name())
    }
}

pub trait ModuleLangExt {
    fn address(&self) -> Option<Address>;
    fn address_equals_to(&self, address: Address, is_completion: bool) -> bool;
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

    fn address_equals_to(&self, address: Address, is_completion: bool) -> bool {
        let module_address = self.address();
        let Some(module_address) = module_address else {
            return false;
        };
        if module_address == address {
            return true;
        }

        let left_numeric = module_address.clone().resolve_to_numeric_address();
        let right_numeric = address.clone().resolve_to_numeric_address();
        tracing::debug!(?left_numeric, ?right_numeric);

        let same_values = match (left_numeric, right_numeric) {
            (Some(left), Some(right)) => left.normalized() == right.short(),
            _ => false,
        };

        if same_values && is_completion {
            // compare named addresses by name in case of the same values for the completion
            match (module_address, address) {
                (Address::Named(left_named), Address::Named(right_named)) => {
                    return left_named == right_named;
                }
                _ => {}
            }
        }

        same_values
    }
}
