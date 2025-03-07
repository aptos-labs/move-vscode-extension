#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Address {
    Named(NamedAddress),
    Value(ValueAddress),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamedAddress {
    name: String,
}

impl NamedAddress {
    pub fn new(name: String) -> Self {
        NamedAddress { name }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValueAddress {
    value: String,
}

impl ValueAddress {
    pub fn new(value: String) -> Self {
        ValueAddress { value }
    }
}
