use crate::types::ty::Ty;
use crate::types::ty::ty_var::TyInfer;
use std::fmt;
use std::fmt::Formatter;
use syntax::ast::Ordering;
use syntax::{AstToken, ast};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum IntegerKind {
    Integer,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
}

pub(crate) const INTEGER_IDENTS: &[&str] = &["u8", "u16", "u32", "u64", "u128", "u256"];

impl IntegerKind {
    pub fn from_suffixed_literal(int_number: ast::IntNumber) -> Option<Self> {
        let lit = int_number.text().to_lowercase();
        let kind = match lit {
            _ if lit.ends_with("u8") => IntegerKind::U8,
            _ if lit.ends_with("u16") => IntegerKind::U16,
            _ if lit.ends_with("u32") => IntegerKind::U32,
            _ if lit.ends_with("u64") => IntegerKind::U64,
            _ if lit.ends_with("u128") => IntegerKind::U128,
            _ if lit.ends_with("u256") => IntegerKind::U256,
            _ => {
                return None;
            }
        };
        Some(kind)
    }

    pub fn is_default(&self) -> bool {
        *self == IntegerKind::Integer
    }
}

impl fmt::Display for IntegerKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            IntegerKind::Integer => "integer",
            _ => &format!("{:?}", self),
        };
        f.write_str(&s.to_lowercase())
    }
}

impl Ty {
    pub fn supports_op(&self, op: ast::BinaryOp) -> bool {
        match op {
            ast::BinaryOp::ArithOp(_) => {
                matches!(self, Ty::Integer(_) | Ty::Infer(_) | Ty::Unknown | Ty::Never)
            }
            ast::BinaryOp::CmpOp(_) => {
                matches!(
                    self,
                    Ty::Integer(_) | Ty::Infer(TyInfer::IntVar(_)) | Ty::Unknown | Ty::Never
                )
            }
            _ => false,
        }
    }

    pub fn supports_arithm_op(&self) -> bool {
        self.supports_op(ast::BinaryOp::ArithOp(ast::ArithOp::Add))
    }

    pub fn supports_ordering(&self) -> bool {
        self.supports_op(ast::BinaryOp::CmpOp(ast::CmpOp::Ord {
            ordering: Ordering::Less,
            strict: true,
        }))
    }
}
