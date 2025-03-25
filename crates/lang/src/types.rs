use crate::db::HirDatabase;
use crate::loc::SyntaxLocFileExt;
use crate::types::ty::Ty;
use crate::InFile;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::{ast, AstNode};

pub mod has_type_params_ext;
pub mod lowering;
pub mod ty;
pub mod substitution;
pub mod inference;

pub(crate) mod render;

mod unification;
mod patterns;
mod expectation;
mod fold;
