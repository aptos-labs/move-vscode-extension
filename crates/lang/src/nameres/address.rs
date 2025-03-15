use std::fmt;
use std::fmt::Formatter;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Address {
    Named(NamedAddr),
    Value(ValueAddr),
}

impl Address {
    pub fn resolve_to_numeric_address(self) -> Option<NumericAddress> {
        match self {
            Address::Named(named_addr) => resolve_named_address(named_addr.name.as_str()),
            Address::Value(value_addr) => Some(value_addr.numeric_address),
        }
    }

    pub fn is_0x0(&self) -> bool {
        match self {
            Address::Value(value_addr) => value_addr.numeric_address.short() == "0x0",
            _ => false,
        }
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

pub fn resolve_named_address(name: &str) -> Option<NumericAddress> {
    if matches!(name, "std" | "aptos_std" | "aptos_framework" | "aptos_token") {
        return Some(NumericAddress {
            value: "0x1".to_string(),
        });
    }
    // todo: get it from AptosWorkspace
    None
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NumericAddress {
    value: String,
}

impl NumericAddress {
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
