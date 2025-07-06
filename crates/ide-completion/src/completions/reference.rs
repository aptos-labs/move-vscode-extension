// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

mod labels;
mod method_or_field;
pub(crate) mod paths;

use crate::completions::Completions;
use crate::completions::reference::labels::add_label_completions;
use crate::completions::reference::method_or_field::add_method_or_field_completions;
use crate::completions::reference::paths::add_path_completions;
use crate::context::{CompletionContext, ReferenceKind};
use crate::item::CompletionItemKind;
use crate::render::new_named_item;
use crate::render::type_owner::render_type_owner;
use lang::node_ext::item::ModuleItemExt;
use std::cell::RefCell;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};

pub(crate) fn add_reference_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    reference_kind: ReferenceKind,
) -> Option<()> {
    let file_id = ctx.position.file_id;
    match reference_kind {
        ReferenceKind::Path { original_path, fake_path } => add_path_completions(
            completions,
            ctx,
            original_path.map(|it| it.in_file(file_id)),
            fake_path,
        ),
        ReferenceKind::DotExpr { receiver_expr } => {
            add_method_or_field_completions(completions, ctx, receiver_expr.in_file(file_id))
        }
        ReferenceKind::Label { fake_label, source_range } => {
            add_label_completions(completions, ctx, fake_label, source_range)
        }
        ReferenceKind::ItemSpecRef { original_item_spec } => {
            add_item_spec_ref_completions(completions, ctx, original_item_spec.in_file(file_id))
        }
        ReferenceKind::StructLitField { original_struct_lit } => {
            add_struct_lit_fields_completions(completions, ctx, original_struct_lit.in_file(file_id))
        }
    }
}

fn add_item_spec_ref_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    item_spec: InFile<ast::ItemSpec>,
) -> Option<()> {
    let acc = &mut completions.borrow_mut();

    acc.add(ctx.new_snippet_item(CompletionItemKind::Keyword, "module $0"));
    acc.add(ctx.new_snippet_item(CompletionItemKind::Keyword, "schema $0"));
    acc.add(ctx.new_snippet_item(CompletionItemKind::Keyword, "fun $0"));

    let module = item_spec.module(ctx.db)?.value;
    for named_item in module.verifiable_items() {
        if let Some(name) = named_item.name() {
            let name = name.as_string();
            let mut comp_item = new_named_item(ctx, &name, named_item.syntax().kind());
            comp_item.insert_snippet(format!("{name} $0"));
            acc.add(comp_item.build(ctx.db));
        }
    }

    Some(())
}

fn add_struct_lit_fields_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    original_struct_lit: InFile<ast::StructLit>,
) -> Option<()> {
    let fields_owner = ctx
        .sema
        .resolve_to_element::<ast::FieldsOwner>(original_struct_lit.map(|it| it.path()))?;

    let acc = &mut completions.borrow_mut();

    for named_field in fields_owner.flat_map(|it| it.named_fields()) {
        if let Some(name) = named_field.value.name() {
            let item = render_type_owner(ctx, &name.as_string(), named_field.map_into());
            acc.add(item.build(ctx.db));
        }
    }
    Some(())
}
