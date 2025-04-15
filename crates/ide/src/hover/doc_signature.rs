use ide_db::RootDatabase;
use lang::Semantics;
use lang::nameres::fq_named_element::ItemFQNameOwner;
use std::fmt::Write;
use syntax::ast::NamedElement;
use syntax::{AstNode, ast, match_ast};

pub trait DocSignatureOwner {
    fn header(&self, sema: &Semantics<'_, RootDatabase>, buffer: &mut String) -> Option<()>;
    fn signature(&self, sema: &Semantics<'_, RootDatabase>, buffer: &mut String) -> Option<()>;
}

impl DocSignatureOwner for ast::AnyNamedElement {
    fn header(&self, _sema: &Semantics<'_, RootDatabase>, buffer: &mut String) -> Option<()> {
        let header = match_ast! {
            match (self.syntax()) {
                ast::Module(it) => it.fq_name()?.address_identifier_text(),
                ast::Item(it) => it.fq_name()?.module_identifier_text(),
                ast::NamedField(it) => it.fields_owner().fq_name()?.identifier_text(),
                ast::Variant(it) => it.enum_().fq_name()?.identifier_text(),
                ast::Const(it) => it.fq_name()?.module_identifier_text(),
                ast::IdentPat(_) => {
                    // no header
                    return None;
                },
                _ => {
                    return None;
                }
            }
        };
        writeln!(buffer, "{}", header).ok()?;

        Some(())
    }

    fn signature(&self, sema: &Semantics<'_, RootDatabase>, buffer: &mut String) -> Option<()> {
        match_ast! {
            match (self.syntax()) {
                ast::Module(it) => generate_module(it, buffer),
                ast::Fun(it) => generate_fun(it, buffer),
                ast::Struct(it) => generate_struct(it, buffer),
                ast::Enum(it) => generate_enum(it, buffer),
                ast::Const(it) => generate_const(it, buffer),
                ast::NamedField(it) => generate_field(it, buffer),
                ast::Variant(it) => generate_enum_variant(it, buffer),
                ast::IdentPat(it) => generate_ident_pat(it, sema, buffer),
                _ => {
                    // do not fail on empty signature
                    Some(())
                }
            }
        }
    }
}

fn generate_module(module: ast::Module, buffer: &mut String) -> Option<()> {
    write!(buffer, "module").ok()?;
    write!(buffer, " ").ok()?;
    write!(buffer, "{}", module.name()?).ok()?;
    Some(())
}

fn generate_fun(fun: ast::Fun, buffer: &mut String) -> Option<()> {
    write!(buffer, "fun").ok()?;
    write!(buffer, " ").ok()?;
    write!(buffer, "{}", fun.name()?).ok()?;
    if let Some(param_list) = fun.param_list() {
        generate_param_list(param_list, buffer);
    }
    Some(())
}

fn generate_param_list(param_list: ast::ParamList, buffer: &mut String) -> Option<()> {
    write!(buffer, "(").ok()?;
    let ps = param_list.params().collect::<Vec<_>>();
    for (i, param) in ps.iter().enumerate() {
        write!(buffer, "{}", param.ident_pat().as_string()).ok()?;
        generate_type_annotation(param.type_(), buffer)?;
        if i != ps.len() - 1 {
            write!(buffer, ", ").ok()?;
        }
    }
    write!(buffer, ")").ok()?;
    Some(())
}

fn generate_const(const_: ast::Const, buffer: &mut String) -> Option<()> {
    write!(buffer, "const {}", const_.name()?.as_string()).ok()?;
    generate_type_annotation(const_.type_(), buffer)?;
    Some(())
}

fn generate_enum(enum_: ast::Enum, buffer: &mut String) -> Option<()> {
    write!(buffer, "enum {} ", enum_.name()?.as_string()).ok()?;
    if let Some(a_list) = enum_.ability_list() {
        generate_abilities_list(a_list, buffer)?;
    }
    write!(buffer, "{{ }}").ok()?;

    Some(())
}

fn generate_struct(struct_: ast::Struct, buffer: &mut String) -> Option<()> {
    write!(buffer, "struct {} ", struct_.name()?.as_string()).ok()?;
    if let Some(a_list) = struct_.ability_list() {
        generate_abilities_list(a_list, buffer)?;
    }
    write!(buffer, "{{ }}").ok()?;

    Some(())
}

fn generate_field(field: ast::NamedField, buffer: &mut String) -> Option<()> {
    write!(buffer, "field {}", field.name()?.as_string()).ok()?;
    generate_type_annotation(field.type_(), buffer)?;
    Some(())
}

fn generate_enum_variant(variant: ast::Variant, buffer: &mut String) -> Option<()> {
    write!(buffer, "variant {}", variant.name()?.as_string()).ok()?;
    Some(())
}

fn generate_ident_pat(
    ident_pat: ast::IdentPat,
    sema: &Semantics<'_, RootDatabase>,
    buffer: &mut String,
) -> Option<()> {
    let owner = ident_pat.owner()?;
    let ident_kind = match owner {
        ast::IdentPatKind::Param(_) => "parameter",
        ast::IdentPatKind::LetStmt(_) => "variable",
        ast::IdentPatKind::SchemaField(_) => "schema field",
    };
    write!(buffer, "{ident_kind} {}", ident_pat.name()?.as_string()).ok()?;

    let ident_pat = sema.wrap_node_infile(ident_pat);
    if let Some(inference) = sema.inference(&ident_pat) {
        let ident_pat_type = inference.get_pat_type(&ast::Pat::IdentPat(ident_pat.value));
        if let Some(ty) = ident_pat_type {
            let rendered_ty = sema.render_ty(ty);
            write!(buffer, ": {}", rendered_ty).ok()?;
        }
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

fn generate_type_annotation(type_: Option<ast::Type>, buf: &mut String) -> Option<()> {
    if let Some(type_) = type_ {
        write!(buf, ": {}", type_.to_string()).ok()?;
    }
    Some(())
}
