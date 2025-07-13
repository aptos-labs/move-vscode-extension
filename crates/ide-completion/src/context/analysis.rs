// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::completions::item_list::ItemListKind;
use crate::context::{COMPLETION_MARKER, CompletionAnalysis, ReferenceKind};
use ide_db::active_parameter::ActiveParameterInfo;
use ide_db::{RootDatabase, active_parameter};
use lang::Semantics;
use lang::types::ty::Ty;
use syntax::SyntaxKind::{FUN, MODULE, SOURCE_FILE, STRUCT_LIT, VALUE_ARG_LIST, VISIBILITY_MODIFIER};
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::{AstNode, SyntaxNode, SyntaxToken, TextRange, TextSize, algo, ast};

pub(crate) struct AnalysisResult {
    pub analysis: CompletionAnalysis,
    pub expected: (Option<Ty>, Option<ast::NameLike>),
}

pub(crate) fn completion_analysis(
    sema: &Semantics<'_, RootDatabase>,
    original_file: &ast::SourceFile,
    fake_file: SyntaxNode,
    original_offset: TextSize,
    original_token: &SyntaxToken,
) -> Option<AnalysisResult> {
    let expected = expected_type_and_name(&sema, original_file.syntax(), original_offset);

    // as we insert after the offset, right biased will *always* pick the identifier no matter
    // if there is an ident already typed or not
    let fake_token = fake_file.token_at_offset(original_offset).right_biased()?;
    if let Some(fake_ref) = fake_token
        .parent_ancestors()
        .find_map(ast::ReferenceElement::cast)
    {
        let analysis = analyze_ref(&fake_ref, original_file, original_offset);
        return analysis.map(|analysis| AnalysisResult { analysis, expected });
    }

    let ident = original_token.clone();
    let mut ident_parent = ident.parent().unwrap();
    if ident_parent.kind().is_error() {
        ident_parent = ident_parent.parent().unwrap();
    }

    let ident_in_parent = ident_parent.child_or_token_at_range(ident.text_range()).unwrap();
    let ident_prev_sibling = ident_in_parent
        .prev_sibling_or_token_no_trivia()
        .map(|it| it.kind());

    let item_list_kind = match ident_parent.kind() {
        SOURCE_FILE => ItemListKind::SourceFile,
        MODULE => {
            let module = ident_parent.cast::<ast::Module>().unwrap();
            // no completions if module has no '{' yet
            let l_curly_token = module.l_curly_token()?;
            // if it's before the '{', then no completions available
            if ident.text_range().end() < l_curly_token.text_range().start() {
                return None;
            }
            ItemListKind::Module
        }
        FUN if ident_prev_sibling == Some(VISIBILITY_MODIFIER) => {
            let fun = ident_parent.cast::<ast::Fun>().unwrap();
            ItemListKind::Function {
                existing_modifiers: fun.modifiers_as_strings(),
            }
        }
        _ => {
            // not an item list
            return None;
        }
    };

    Some(AnalysisResult {
        analysis: CompletionAnalysis::Item(item_list_kind),
        expected,
    })
}

fn analyze_ref(
    fake_ref: &ast::ReferenceElement,
    original_file: &ast::SourceFile,
    original_offset: TextSize,
) -> Option<CompletionAnalysis> {
    let reference_kind = match fake_ref {
        ast::ReferenceElement::Path(fake_path) => {
            // check for struct lit field
            analyze_path(fake_path, original_file, original_offset)
        }
        ast::ReferenceElement::DotExpr(_) => {
            let original_receiver_expr = original_file
                .find_node_at_offset::<ast::DotExpr>(original_offset)?
                .receiver_expr();
            Some(ReferenceKind::DotExpr { original_receiver_expr })
        }
        ast::ReferenceElement::Label(fake_label) => {
            let fake_range = fake_label.syntax().text_range();
            Some(ReferenceKind::Label {
                fake_label: fake_label.clone(),
                source_range: TextRange::new(
                    fake_range.start(),
                    fake_range.end() - TextSize::of(COMPLETION_MARKER),
                ),
            })
        }
        ast::ReferenceElement::ItemSpecRef(fake_item_spec_ref) => {
            // spec keyword location will be the same in the original file
            let fake_spec_kw = fake_item_spec_ref.item_spec().spec_token()?;
            let original_spec_kw = original_file.find_original_token(fake_spec_kw)?;
            let original_item_spec = original_spec_kw.parent()?.cast::<ast::ItemSpec>()?;
            Some(ReferenceKind::ItemSpecRef { original_item_spec })
        }
        ast::ReferenceElement::IdentPat(fake_ident_pat) => {
            let fake_struct_pat_field =
                fake_ident_pat.syntax().parent_of_type::<ast::StructPatField>()?;
            if !fake_struct_pat_field.is_shorthand() {
                return None;
            }
            let fake_struct_pat = fake_struct_pat_field.struct_pat();
            let original_struct_pat = original_file.find_original_node(fake_struct_pat)?;
            Some(ReferenceKind::StructPatField { original_struct_pat })
        }
        _ => None,
    };
    reference_kind.map(|kind| CompletionAnalysis::Reference(kind))
}

fn analyze_path(
    fake_path: &ast::Path,
    original_file: &ast::SourceFile,
    original_offset: TextSize,
) -> Option<ReferenceKind> {
    // check for struct lit field
    if let Some(fake_path_expr) = fake_path.root_path().path_expr()
        && let Some(fake_struct_lit_field) =
        fake_path_expr.syntax().parent_of_type::<ast::StructLitField>()
        // S { val/*caret*/ }
        && fake_struct_lit_field.is_shorthand()
    {
        let fake_struct_lit = fake_struct_lit_field.struct_lit();
        let original_struct_lit = original_file.find_original_node(fake_struct_lit)?;
        return Some(ReferenceKind::StructLitField { original_struct_lit });
    }

    let original_path = original_file.find_node_at_offset::<ast::Path>(original_offset);
    Some(ReferenceKind::Path {
        original_path,
        fake_path: fake_path.clone(),
    })
}

fn expected_type_and_name<'db>(
    sema: &Semantics<'db, RootDatabase>,
    original_file: &SyntaxNode,
    offset: TextSize,
) -> (Option<Ty>, Option<ast::NameLike>) {
    for ancestor in algo::ancestors_at_offset(original_file, offset) {
        if ancestor.kind() == VALUE_ARG_LIST {
            let active_param = ActiveParameterInfo::at_offset(sema, original_file, offset);
            return active_param
                .map(|active| {
                    let name = active.ident().map(ast::NameLike::Name);
                    (active.ty, name)
                })
                .unwrap_or((None, None));
        }
        if ancestor.kind() == STRUCT_LIT {
            let struct_lit = sema.wrap_node_infile(ancestor.cast::<ast::StructLit>().unwrap());
            let msl = struct_lit.is_msl();
            if let Some((fields_owner, Some(active_field_name))) =
                active_parameter::fields_owner_for_struct_lit(sema, struct_lit, offset)
            {
                if let Some(named_field) =
                    fields_owner.and_then(|it| it.field_by_name(&active_field_name))
                {
                    let name = named_field.value.field_name();
                    let ty = named_field
                        .and_then(|it| it.type_())
                        .map(|field_type| sema.lower_type(field_type, msl));
                    return (ty, Some(name.into()));
                }
            }
        }
    }
    (None, None)
}
