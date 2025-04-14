use lang::nameres::fq_named_element::ItemFQNameOwner;
use std::fmt::Write;
use syntax::SyntaxKind::*;
use syntax::ast::NamedElement;
use syntax::{AstNode, ast};

pub trait DocSignatureOwner {
    fn header(&self, buffer: &mut String) -> Option<()>;
    fn signature(&self, buffer: &mut String) -> Option<()>;
}

impl DocSignatureOwner for ast::AnyNamedElement {
    fn header(&self, buffer: &mut String) -> Option<()> {
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
        writeln!(buffer, "{}", header).ok()?;

        Some(())
    }

    fn signature(&self, buffer: &mut String) -> Option<()> {
        match self.syntax().kind() {
            FUN => generate_fun(self.cast_into::<ast::Fun>()?, buffer),
            STRUCT | ENUM => generate_struct_or_enum(self.cast_into::<ast::StructOrEnum>()?, buffer),
            NAMED_FIELD => generate_field(self.cast_into::<ast::NamedField>()?, buffer),
            _ => None,
        }
    }
}

fn generate_fun(fun: ast::Fun, buffer: &mut String) -> Option<()> {
    write!(buffer, "fun").ok()?;
    write!(buffer, " ").ok()?;
    write!(buffer, "{}", fun.name()?).ok()?;
    write!(buffer, "()").ok()?;
    Some(())
}

fn generate_struct_or_enum(struct_or_enum: ast::StructOrEnum, buffer: &mut String) -> Option<()> {
    let name = struct_or_enum.name()?.as_string();

    match struct_or_enum {
        ast::StructOrEnum::Struct(s) => {
            write!(buffer, "struct {name} ").ok()?;
            if let Some(a_list) = s.ability_list() {
                generate_abilities_list(a_list, buffer)?;
            }
            write!(buffer, "{{ }}").ok()?;
        }
        ast::StructOrEnum::Enum(e) => {
            write!(buffer, "enum {name} ").ok()?;
            if let Some(a_list) = e.ability_list() {
                generate_abilities_list(a_list, buffer)?;
            }
            write!(buffer, "{{ }}").ok()?;
        }
    };

    Some(())
}

fn generate_field(field: ast::NamedField, buffer: &mut String) -> Option<()> {
    write!(buffer, "field {}", field.name()?.as_string()).ok()?;
    if let Some(field_type) = field.type_() {
        generate_type_annotation(field_type, buffer)?;
    }
    Some(())
}

fn generate_abilities_list(abilities_list: ast::AbilityList, buffer: &mut String) -> Option<()> {
    write!(buffer, "has ").ok()?;
    let abs = abilities_list.abilities().collect::<Vec<_>>();
    for (i, ability) in abs.iter().enumerate() {
        let ability_text = ability.ident_token().to_string();
        write!(buffer, "{ability_text}").ok()?;
        if i != abs.len() - 1 {
            write!(buffer, ", ").ok()?;
        }
    }
    write!(buffer, " ").ok()?;
    Some(())
}

fn generate_type_annotation(type_: ast::Type, buffer: &mut String) -> Option<()> {
    write!(buffer, ": {}", type_.to_string()).ok()?;
    Some(())
}
