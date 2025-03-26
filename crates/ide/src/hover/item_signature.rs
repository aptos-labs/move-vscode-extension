use lang::nameres::fq_named_element::ItemFQNameOwner;
use std::fmt::Write;
use syntax::SyntaxKind::*;
use syntax::ast::NamedElement;
use syntax::{AstNode, ast};

pub trait DocSignatureOwner {
    fn owner(&self, buffer: &mut String) -> Option<()>;
    fn signature(&self, buffer: &mut String) -> Option<()>;
}

impl DocSignatureOwner for ast::AnyNamedElement {
    fn owner(&self, buffer: &mut String) -> Option<()> {
        let header = match self.syntax().kind() {
            NAMED_FIELD => {
                let fields_owner = self.cast_into::<ast::NamedField>()?.fields_owner();
                fields_owner.fq_name()?.identifier_text()
            }
            FUN | SPEC_FUN | SPEC_INLINE_FUN | STRUCT | ENUM | CONST | SCHEMA => {
                let item = self.cast_into::<ast::AnyNamedElement>()?;
                item.fq_name()?.module_identifier_text()
            }
            _ => {
                return None;
            }
        };
        write!(buffer, "{}", header).ok()?;

        Some(())
    }

    fn signature(&self, buffer: &mut String) -> Option<()> {
        match self.syntax().kind() {
            FUN => {
                let fun = self.cast_into::<ast::Fun>()?;
                write!(buffer, "fun").ok()?;
                write!(buffer, " ").ok()?;
                write!(buffer, "{}", fun.name()?).ok()?;
            }
            _ => {
                return None;
            }
        };

        Some(())
    }
}
