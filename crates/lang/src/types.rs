pub mod has_type_params_ext;
pub mod inference;
pub mod lowering;
pub mod substitution;
pub mod ty;

pub(crate) mod render;

mod expectation;
pub mod fold;
mod patterns;
mod unification;
