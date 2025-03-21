use crate::db::HirDatabase;
use crate::loc::SyntaxLocExt;
use crate::types::ty::Ty;
use crate::InFile;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::{ast, AstNode};

mod expectation;
mod fold;
mod has_type_params_ext;
pub(crate) mod inference;
pub(crate) mod lowering;
mod patterns;
pub(crate) mod render;
mod substitution;
pub mod ty;
mod unification;
