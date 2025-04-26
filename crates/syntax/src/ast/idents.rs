use std::cell::OnceCell;
use std::sync::LazyLock;
use stdx::itertools::Itertools;

pub const INTEGER_IDENTS: &[&str] = &["u8", "u16", "u32", "u64", "u128", "u256", "num"];

pub static PRIMITIVE_TYPES: LazyLock<Vec<&str>> =
    LazyLock::new(|| [&["bool", "address", "signer", "vector"], INTEGER_IDENTS].concat());
