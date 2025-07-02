use crate::ast::operators::{ArithOp, BinaryOp, CmpOp, LogicOp, Ordering};
use crate::ast::support;
use crate::{AstNode, SyntaxToken, T, ast};

impl ast::BinExpr {
    // todo: not optional
    pub fn op_details(&self) -> Option<(SyntaxToken, BinaryOp)> {
        self.syntax()
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .find_map(|c| {
                #[rustfmt::skip]
                let bin_op = match c.kind() {
                    T![||] => BinaryOp::LogicOp(LogicOp::Or),
                    T![&&] => BinaryOp::LogicOp(LogicOp::And),

                    T![==] => BinaryOp::CmpOp(CmpOp::Eq { negated: false }),
                    T![!=] => BinaryOp::CmpOp(CmpOp::Eq { negated: true }),
                    T![<=] => BinaryOp::CmpOp(CmpOp::Ord { ordering: Ordering::Less,    strict: false }),
                    T![>=] => BinaryOp::CmpOp(CmpOp::Ord { ordering: Ordering::Greater, strict: false }),
                    T![<]  => BinaryOp::CmpOp(CmpOp::Ord { ordering: Ordering::Less,    strict: true }),
                    T![>]  => BinaryOp::CmpOp(CmpOp::Ord { ordering: Ordering::Greater, strict: true }),

                    T![+]  => BinaryOp::ArithOp(ArithOp::Add),
                    T![*]  => BinaryOp::ArithOp(ArithOp::Mul),
                    T![-]  => BinaryOp::ArithOp(ArithOp::Sub),
                    T![/]  => BinaryOp::ArithOp(ArithOp::Div),
                    T![%]  => BinaryOp::ArithOp(ArithOp::Rem),
                    T![<<] => BinaryOp::ArithOp(ArithOp::Shl),
                    T![>>] => BinaryOp::ArithOp(ArithOp::Shr),
                    T![^]  => BinaryOp::ArithOp(ArithOp::BitXor),
                    T![|]  => BinaryOp::ArithOp(ArithOp::BitOr),
                    T![&]  => BinaryOp::ArithOp(ArithOp::BitAnd),

                    T![=]   => BinaryOp::Assignment { op: None },
                    T![+=]  => BinaryOp::Assignment { op: Some(ArithOp::Add) },
                    T![*=]  => BinaryOp::Assignment { op: Some(ArithOp::Mul) },
                    T![-=]  => BinaryOp::Assignment { op: Some(ArithOp::Sub) },
                    T![/=]  => BinaryOp::Assignment { op: Some(ArithOp::Div) },
                    T![%=]  => BinaryOp::Assignment { op: Some(ArithOp::Rem) },
                    T![<<=] => BinaryOp::Assignment { op: Some(ArithOp::Shl) },
                    T![>>=] => BinaryOp::Assignment { op: Some(ArithOp::Shr) },
                    T![^=]  => BinaryOp::Assignment { op: Some(ArithOp::BitXor) },
                    T![|=]  => BinaryOp::Assignment { op: Some(ArithOp::BitOr) },
                    T![&=]  => BinaryOp::Assignment { op: Some(ArithOp::BitAnd) },

                    _ => return None,
                };
                Some((c, bin_op))
            })
    }

    pub fn op_kind(&self) -> Option<BinaryOp> {
        self.op_details().map(|t| t.1)
    }

    pub fn op_token(&self) -> Option<SyntaxToken> {
        self.op_details().map(|t| t.0)
    }

    pub fn lhs(&self) -> Option<ast::Expr> {
        support::children(self.syntax()).next()
    }

    pub fn rhs(&self) -> Option<ast::Expr> {
        support::children(self.syntax()).nth(1)
    }

    pub fn unpack(&self) -> Option<(ast::Expr, (SyntaxToken, BinaryOp), Option<ast::Expr>)> {
        #[rustfmt::skip]
        let (Some(lhs), Some(op), rhs) =
            (self.lhs(), self.op_details(), self.rhs())
        else {
            return None;
        };
        Some((lhs, op, rhs))
    }

    pub fn sub_exprs(&self) -> (Option<ast::Expr>, Option<ast::Expr>) {
        let mut children = support::children(self.syntax());
        let first = children.next();
        let second = children.next();
        (first, second)
    }
}
