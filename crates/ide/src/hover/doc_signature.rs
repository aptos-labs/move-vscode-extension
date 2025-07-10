// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use ide_db::RootDatabase;
use lang::Semantics;
use std::fmt::Write;
use stdx::format_to;
use syntax::{AstNode, ast, match_ast};

pub trait DocSignatureOwner {
    fn header(&self, sema: &Semantics<'_, RootDatabase>, buffer: &mut String) -> Option<()>;
    fn signature(&self, sema: &Semantics<'_, RootDatabase>, buffer: &mut String) -> Option<()>;
}

impl DocSignatureOwner for ast::NamedElement {
    fn header(&self, sema: &Semantics<'_, RootDatabase>, buffer: &mut String) -> Option<()> {
        let header = match_ast! {
            match (self.syntax()) {
                ast::Module(it) => sema.fq_name_for_item(it)?.address_identifier_text(),
                ast::Item(it) => sema.fq_name_for_item(it)?.module_identifier_text(),
                ast::SpecInlineFun(it) => sema.fq_name_for_item(it)?.module_identifier_text(),
                ast::NamedField(it) => sema.fq_name_for_item(it.fields_owner())?.fq_identifier_text(),
                ast::Variant(it) => sema.fq_name_for_item(it.enum_())?.fq_identifier_text(),
                ast::Const(it) => sema.fq_name_for_item(it)?.module_identifier_text(),
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

    fn signature(&self, sema: &Semantics<'_, RootDatabase>, buf: &mut String) -> Option<()> {
        match_ast! {
            match (self.syntax()) {
                ast::Module(it) => generate_module(buf, it),
                ast::AnyFun(it) => generate_any_fun(buf, it),
                ast::Struct(it) => generate_struct(buf, it),
                ast::Enum(it) => generate_enum(buf, it),
                ast::Const(it) => generate_const(buf, it),
                ast::NamedField(it) => {
                    format_to!(buf, "field ");
                    generate_field(buf, it)
                },
                ast::Variant(it) => {
                    format_to!(buf, "variant ");
                    generate_enum_variant(it, buf, true)
                },
                ast::IdentPat(it) => generate_ident_pat(buf, sema, it),
                _ => {
                    // do not fail on empty signature
                    Some(())
                }
            }
        }
    }
}

fn generate_module(buf: &mut String, module: ast::Module) -> Option<()> {
    let module_name = module.name()?;
    format_to!(buf, "module {module_name}");
    Some(())
}

fn generate_any_fun(buf: &mut String, any_fun: ast::AnyFun) -> Option<()> {
    let fun_kw = match any_fun {
        ast::AnyFun::Fun(_) => "fun",
        ast::AnyFun::SpecFun(_) | ast::AnyFun::SpecInlineFun(_) => "spec fun",
    };
    let fun_name = any_fun.name()?;
    format_to!(buf, "{fun_kw} {fun_name}");

    if let Some(param_list) = any_fun.param_list() {
        separated_list(
            buf,
            param_list.params().collect(),
            "(",
            ")",
            ", ",
            false,
            |buf, param| {
                format_to!(buf, "{}", param.ident_name());
                generate_type_annotation(buf, param.type_())?;
                Some(())
            },
        )
    }
    generate_type_annotation(buf, any_fun.return_type());
    Some(())
}

fn generate_const(buf: &mut String, const_: ast::Const) -> Option<()> {
    let const_name = const_.name()?.as_string();
    format_to!(buf, "const {const_name}");
    generate_type_annotation(buf, const_.type_())?;
    Some(())
}

fn generate_enum(buf: &mut String, enum_: ast::Enum) -> Option<()> {
    format_to!(buf, "enum {}", enum_.name()?.as_string());

    if let Some(a_list) = enum_.ability_list() {
        format_to!(buf, " ");
        generate_abilities_list(buf, a_list)?;
        // format_to!(buf, " ");
    }

    separated_list(buf, enum_.variants(), " {", "}", ",", true, |buf, variant| {
        generate_enum_variant(variant, buf, false)
    });

    Some(())
}

fn generate_struct(buf: &mut String, struct_: ast::Struct) -> Option<()> {
    let struct_name = struct_.name()?.as_string();
    format_to!(buf, "struct {struct_name}");

    if let Some(a_list) = struct_.ability_list() {
        format_to!(buf, " ");
        generate_abilities_list(buf, a_list)?;
        // format_to!(buf, " ");
    }

    let field_list = struct_.field_list()?;
    generate_field_list(buf, field_list, true);

    Some(())
}

fn generate_field_list(buf: &mut String, field_list: ast::FieldList, verbose: bool) -> Option<()> {
    match field_list {
        ast::FieldList::NamedFieldList(_) => {
            format_to!(buf, " {{ ... }}");
            // if !verbose {
            // } else {
            //     separated_list(
            //         buf,
            //         named_field_list.fields().collect(),
            //         " {",
            //         "}",
            //         ",",
            //         true,
            //         generate_field,
            //     );
            // }
        }
        ast::FieldList::TupleFieldList(tuple_field_list) => {
            if !verbose {
                format_to!(buf, "(...)");
            } else {
                separated_list(
                    buf,
                    tuple_field_list.fields().collect(),
                    "(",
                    ")",
                    ", ",
                    false,
                    |buf, tuple_field| generate_type(buf, tuple_field.type_()),
                );
            }
        }
    }
    Some(())
}

fn generate_field(buf: &mut String, field: ast::NamedField) -> Option<()> {
    format_to!(buf, "{}", field.field_name().as_string());
    generate_type_annotation(buf, field.type_())?;
    Some(())
}

fn generate_enum_variant(variant: ast::Variant, buf: &mut String, verbose: bool) -> Option<()> {
    format_to!(buf, "{}", variant.name()?.as_string());

    let field_list = variant.field_list()?;
    generate_field_list(buf, field_list, verbose);

    Some(())
}

fn generate_ident_pat(
    buf: &mut String,
    sema: &Semantics<'_, RootDatabase>,
    ident_pat: ast::IdentPat,
) -> Option<()> {
    let ident_kind = ident_pat.ident_owner()?.kind();
    let ident_name = ident_pat.name()?.as_string();
    format_to!(buf, "{ident_kind} {ident_name}");

    let ident_pat = sema.wrap_node_infile(ident_pat);
    if let Some(ident_pat_ty) = sema.get_ident_pat_type(&ident_pat, false) {
        let rendered_ty = sema.render_ty(&ident_pat_ty);
        format_to!(buf, ": {}", rendered_ty);
    }

    Some(())
}

fn generate_abilities_list(buf: &mut String, abilities_list: ast::AbilityList) -> Option<()> {
    let abilities = abilities_list.abilities().collect::<Vec<_>>();
    separated_list(buf, abilities, "has ", "", ", ", false, |buf, ability| {
        format_to!(buf, "{}", ability.ident_token().to_string());
        Some(())
    });
    Some(())
}

fn generate_type_annotation(buf: &mut String, type_: Option<ast::Type>) -> Option<()> {
    if let Some(type_) = type_ {
        format_to!(buf, ": ");
        generate_type(buf, Some(type_));
    }
    Some(())
}

fn generate_type(buf: &mut String, type_: Option<ast::Type>) -> Option<()> {
    let type_ = type_?;
    format_to!(buf, "{}", type_.to_string());
    Some(())
}

fn separated_list<T>(
    buf: &mut String,
    elements: Vec<T>,
    start: &str,
    end: &str,
    sep: &str,
    one_per_line: bool,
    f: impl Fn(&mut String, T) -> Option<()>,
) {
    format_to!(buf, "{}", start);
    if one_per_line {
        format_to!(buf, "\n");
    }
    let elements_len = elements.len();
    for (i, element) in elements.into_iter().enumerate() {
        if one_per_line {
            // indent
            format_to!(buf, "    ");
        }
        let _ = f(buf, element);
        if i != elements_len - 1 {
            format_to!(buf, "{}", sep);
        }
        if one_per_line {
            format_to!(buf, "\n");
        }
    }
    format_to!(buf, "{}", end);
}
