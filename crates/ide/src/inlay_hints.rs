// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

mod ident_pat;

use crate::NavigationTarget;
use ide_db::RootDatabase;
use ide_db::text_edit::{TextEdit, TextEditBuilder};
use itertools::Itertools;
use lang::Semantics;
use lang::types::render::HirWrite;
use lang::types::ty::Ty;
use std::collections::HashSet;
use std::{fmt, mem};
use syntax::files::{FileRange, InFile, InFileExt};
use syntax::{AstNode, SyntaxNode, TextRange, TextSize, WalkEvent, ast, match_ast};
use vfs::FileId;

#[tracing::instrument(level = "info", skip_all)]
pub(crate) fn inlay_hints(
    db: &RootDatabase,
    file_id: FileId,
    range_limit: Option<TextRange>,
    config: &InlayHintsConfig,
) -> Vec<InlayHint> {
    let sema = Semantics::new(db, file_id);
    let file = sema.parse(file_id);
    let file = file.syntax();

    let mut acc = Vec::new();
    let mut preorder = file.preorder();
    while let Some(event) = preorder.next() {
        // FIXME: This can miss some hints that require the parent of the range to calculate
        if matches!(
            (&event, range_limit), (WalkEvent::Enter(node), Some(range))
            if range.intersect(node.text_range()).is_none())
        {
            preorder.skip_subtree();
            continue;
        }
        if let WalkEvent::Enter(node) = event {
            hints(&mut acc, &sema, config, file_id, node);
        }
    }
    if let Some(range_limit) = range_limit {
        acc.retain(|hint| range_limit.contains_range(hint.range));
    }
    acc
}

fn hints(
    hints: &mut Vec<InlayHint>,
    sema: &Semantics<'_, RootDatabase>,
    config: &InlayHintsConfig,
    file_id: FileId,
    node: SyntaxNode,
) {
    match_ast! {
        match node {
            ast::IdentPat(it) => {
                ident_pat::hints(hints, sema, config, &it.in_file(file_id));
                Some(())
            },
            _ => Some(()),
        }
    };
}

pub(crate) fn inlay_hints_resolve(
    db: &RootDatabase,
    file_id: FileId,
    resolve_range: TextRange,
    hash: u64,
    config: &InlayHintsConfig,
    hasher: impl Fn(&InlayHint) -> u64,
) -> Option<InlayHint> {
    let _p = tracing::info_span!("inlay_hints_resolve").entered();
    let sema = Semantics::new(db, file_id);
    let file = sema.parse(file_id);
    let file = file.syntax();

    let mut acc = Vec::new();
    let mut preorder = file.preorder();
    while let Some(event) = preorder.next() {
        // FIXME: This can miss some hints that require the parent of the range to calculate
        if matches!(&event, WalkEvent::Enter(node) if resolve_range.intersect(node.text_range()).is_none())
        {
            preorder.skip_subtree();
            continue;
        }
        if let WalkEvent::Enter(node) = event {
            hints(&mut acc, &sema, config, file_id, node);
        }
    }
    acc.into_iter().find(|hint| hasher(hint) == hash)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlayHintsConfig {
    pub render_colons: bool,
    pub type_hints: bool,
    // pub parameter_hints: bool,
    // pub generic_parameter_hints: GenericParameterHints,
    // pub chaining_hints: bool,
    pub hide_closure_parameter_hints: bool,
    // pub max_length: Option<usize>,
    // pub closing_brace_hints_min_lines: Option<usize>,
    pub fields_to_resolve: InlayFieldsToResolve,
}

impl InlayHintsConfig {
    fn lazy_text_edit(&self, finish: impl FnOnce() -> TextEdit) -> LazyProperty<TextEdit> {
        if self.fields_to_resolve.resolve_text_edits {
            LazyProperty::Lazy
        } else {
            let edit = finish();
            stdx::never!(edit.is_empty(), "inlay hint produced an empty text edit");
            LazyProperty::Computed(edit)
        }
    }

    fn lazy_tooltip(&self, finish: impl FnOnce() -> InlayTooltip) -> LazyProperty<InlayTooltip> {
        if self.fields_to_resolve.resolve_hint_tooltip && self.fields_to_resolve.resolve_label_tooltip {
            LazyProperty::Lazy
        } else {
            let tooltip = finish();
            stdx::never!(
                match &tooltip {
                    InlayTooltip::String(s) => s,
                    InlayTooltip::Markdown(s) => s,
                }
                .is_empty(),
                "inlay hint produced an empty tooltip"
            );
            LazyProperty::Computed(tooltip)
        }
    }

    /// This always reports a resolvable location, so only use this when it is very likely for a
    /// location link to actually resolve but where computing `finish` would be costly.
    fn lazy_location_opt(
        &self,
        finish: impl FnOnce() -> Option<FileRange>,
    ) -> Option<LazyProperty<FileRange>> {
        if self.fields_to_resolve.resolve_label_location {
            Some(LazyProperty::Lazy)
        } else {
            finish().map(LazyProperty::Computed)
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct InlayFieldsToResolve {
    pub resolve_text_edits: bool,
    pub resolve_hint_tooltip: bool,
    pub resolve_label_tooltip: bool,
    pub resolve_label_location: bool,
    pub resolve_label_command: bool,
}

impl InlayFieldsToResolve {
    pub fn from_client_capabilities(client_capability_fields: &HashSet<&str>) -> Self {
        Self {
            resolve_text_edits: client_capability_fields.contains("textEdits"),
            resolve_hint_tooltip: client_capability_fields.contains("tooltip"),
            resolve_label_tooltip: client_capability_fields.contains("label.tooltip"),
            resolve_label_location: client_capability_fields.contains("label.location"),
            resolve_label_command: client_capability_fields.contains("label.command"),
        }
    }

    pub const fn empty() -> Self {
        Self {
            resolve_text_edits: false,
            resolve_hint_tooltip: false,
            resolve_label_tooltip: false,
            resolve_label_location: false,
            resolve_label_command: false,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum InlayKind {
    Adjustment,
    BindingMode,
    Chaining,
    ClosingBrace,
    ClosureCapture,
    Discriminant,
    GenericParamList,
    Lifetime,
    Parameter,
    GenericParameter,
    Type,
    Drop,
    RangeExclusive,
    ExternUnsafety,
}

#[derive(Debug, Hash)]
pub enum InlayHintPosition {
    Before,
    After,
}

#[derive(Debug)]
pub struct InlayHint {
    /// The text range this inlay hint applies to.
    pub range: TextRange,
    pub position: InlayHintPosition,
    pub pad_left: bool,
    pub pad_right: bool,
    /// The kind of this inlay hint.
    pub kind: InlayKind,
    /// The actual label to show in the inlay hint.
    pub label: InlayHintLabel,
    /// Text edit to apply when "accepting" this inlay hint.
    pub text_edit: Option<LazyProperty<TextEdit>>,
    /// Range to recompute inlay hints when trying to resolve for this hint. If this is none, the
    /// hint does not support resolving.
    pub resolve_parent: Option<TextRange>,
}

/// A type signaling that a value is either computed, or is available for computation.
#[derive(Clone, Debug)]
pub enum LazyProperty<T> {
    Computed(T),
    Lazy,
}

impl<T> LazyProperty<T> {
    pub fn computed(self) -> Option<T> {
        match self {
            LazyProperty::Computed(it) => Some(it),
            _ => None,
        }
    }

    pub fn is_lazy(&self) -> bool {
        matches!(self, Self::Lazy)
    }
}

impl std::hash::Hash for InlayHint {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.range.hash(state);
        self.position.hash(state);
        self.pad_left.hash(state);
        self.pad_right.hash(state);
        self.kind.hash(state);
        self.label.hash(state);
        mem::discriminant(&self.text_edit).hash(state);
    }
}

impl InlayHint {
    fn closing_paren_after(kind: InlayKind, range: TextRange) -> InlayHint {
        InlayHint {
            range,
            kind,
            label: InlayHintLabel::from(")"),
            text_edit: None,
            position: InlayHintPosition::After,
            pad_left: false,
            pad_right: false,
            resolve_parent: None,
        }
    }
}

#[derive(Debug, Hash)]
pub enum InlayTooltip {
    String(String),
    Markdown(String),
}

#[derive(Default, Hash)]
pub struct InlayHintLabel {
    pub parts: Vec<InlayHintLabelPart>,
}

impl InlayHintLabel {
    pub fn simple(
        s: impl Into<String>,
        tooltip: Option<LazyProperty<InlayTooltip>>,
        linked_location: Option<LazyProperty<FileRange>>,
    ) -> InlayHintLabel {
        InlayHintLabel {
            parts: vec![InlayHintLabelPart {
                text: s.into(),
                linked_location,
                tooltip,
            }],
        }
    }

    pub fn prepend_str(&mut self, s: &str) {
        match &mut *self.parts {
            [
                InlayHintLabelPart {
                    text,
                    linked_location: None,
                    tooltip: None,
                },
                ..,
            ] => text.insert_str(0, s),
            _ => self.parts.insert(
                0,
                InlayHintLabelPart {
                    text: s.into(),
                    linked_location: None,
                    tooltip: None,
                },
            ),
        }
    }

    pub fn append_str(&mut self, s: &str) {
        match &mut *self.parts {
            [
                ..,
                InlayHintLabelPart {
                    text,
                    linked_location: None,
                    tooltip: None,
                },
            ] => text.push_str(s),
            _ => self.parts.push(InlayHintLabelPart {
                text: s.into(),
                linked_location: None,
                tooltip: None,
            }),
        }
    }

    pub fn append_part(&mut self, part: InlayHintLabelPart) {
        if part.linked_location.is_none() && part.tooltip.is_none() {
            if let Some(InlayHintLabelPart {
                text,
                linked_location: None,
                tooltip: None,
            }) = self.parts.last_mut()
            {
                text.push_str(&part.text);
                return;
            }
        }
        self.parts.push(part);
    }
}

impl From<String> for InlayHintLabel {
    fn from(s: String) -> Self {
        Self {
            parts: vec![InlayHintLabelPart {
                text: s,
                linked_location: None,
                tooltip: None,
            }],
        }
    }
}

impl From<&str> for InlayHintLabel {
    fn from(s: &str) -> Self {
        Self {
            parts: vec![InlayHintLabelPart {
                text: s.into(),
                linked_location: None,
                tooltip: None,
            }],
        }
    }
}

impl fmt::Display for InlayHintLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.parts.iter().map(|part| &part.text).format(""))
    }
}

impl fmt::Debug for InlayHintLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.parts).finish()
    }
}

pub struct InlayHintLabelPart {
    pub text: String,
    /// Source location represented by this label part. The client will use this to fetch the part's
    /// hover tooltip, and Ctrl+Clicking the label part will navigate to the definition the location
    /// refers to (not necessarily the location itself).
    /// When setting this, no tooltip must be set on the containing hint, or VS Code will display
    /// them both.
    pub linked_location: Option<LazyProperty<FileRange>>,
    /// The tooltip to show when hovering over the inlay hint, this may invoke other actions like
    /// hover requests to show.
    pub tooltip: Option<LazyProperty<InlayTooltip>>,
}

impl std::hash::Hash for InlayHintLabelPart {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.text.hash(state);
        self.linked_location.is_some().hash(state);
        self.tooltip.is_some().hash(state);
    }
}

impl fmt::Debug for InlayHintLabelPart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self {
                text,
                linked_location: None,
                tooltip: None | Some(LazyProperty::Lazy),
            } => text.fmt(f),
            Self {
                text,
                linked_location,
                tooltip,
            } => f
                .debug_struct("InlayHintLabelPart")
                .field("text", text)
                .field("linked_location", linked_location)
                .field(
                    "tooltip",
                    &tooltip.as_ref().map_or("", |it| match it {
                        LazyProperty::Computed(
                            InlayTooltip::String(it) | InlayTooltip::Markdown(it),
                        ) => it,
                        LazyProperty::Lazy => "",
                    }),
                )
                .finish(),
        }
    }
}

#[derive(Debug)]
struct InlayHintLabelBuilder<'a> {
    db: &'a RootDatabase,
    result: InlayHintLabel,
    last_part: String,
    client_can_lazy_resolve: bool,
    location: Option<LazyProperty<FileRange>>,
}

impl fmt::Write for InlayHintLabelBuilder<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.last_part.write_str(s)
    }
}

impl HirWrite for InlayHintLabelBuilder<'_> {
    fn start_location_link(&mut self, named_item: InFile<ast::NamedElement>) -> Option<()> {
        stdx::never!(self.location.is_some(), "location link is already started");
        self.make_new_part();
        self.location = Some(if self.client_can_lazy_resolve {
            LazyProperty::Lazy
        } else {
            LazyProperty::Computed({
                let nav_target = NavigationTarget::from_named_item(named_item)?;
                FileRange {
                    file_id: nav_target.file_id,
                    range: nav_target.focus_or_full_range(),
                }
            })
        });
        Some(())
    }

    fn end_location_link(&mut self) {
        self.make_new_part();
    }
}

impl InlayHintLabelBuilder<'_> {
    fn make_new_part(&mut self) {
        let text = mem::take(&mut self.last_part);
        if !text.is_empty() {
            self.result.parts.push(InlayHintLabelPart {
                text,
                linked_location: self.location.take(),
                tooltip: None,
            });
        }
    }

    fn finish(mut self) -> InlayHintLabel {
        self.make_new_part();
        self.result
    }
}

fn label_of_ty(
    sema: &Semantics<'_, RootDatabase>,
    config: &InlayHintsConfig,
    file_id: FileId,
    ty: &Ty,
) -> Option<InlayHintLabel> {
    let mut label_builder = InlayHintLabelBuilder {
        db: sema.db,
        last_part: String::new(),
        location: None,
        result: InlayHintLabel::default(),
        client_can_lazy_resolve: config.fields_to_resolve.resolve_label_location,
    };
    sema.render_ty_truncated_to(ty, file_id, &mut label_builder)
        .ok()?;
    // label_builder
    //     .write_str(&sema.render_ty_truncated(ty, file_id))
    //     .unwrap();
    Some(label_builder.finish())
}

fn ty_to_text_edit(
    sema: &Semantics<'_, RootDatabase>,
    config: &InlayHintsConfig,
    ty: Ty,
    offset_to_insert_ty: TextSize,
    additional_edits: &dyn Fn(&mut TextEditBuilder),
    prefix: impl Into<String>,
) -> Option<LazyProperty<TextEdit>> {
    // FIXME: Limit the length and bail out on excess somehow?
    let rendered = sema.render_ty(&ty);
    Some(config.lazy_text_edit(|| {
        let mut builder = TextEdit::builder();
        builder.insert(offset_to_insert_ty, prefix.into());
        builder.insert(offset_to_insert_ty, rendered);

        additional_edits(&mut builder);

        builder.finish()
    }))
}
