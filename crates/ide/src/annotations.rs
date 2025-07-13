// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::NavigationTarget;
use crate::goto_specification::goto_specification;
use base_db::inputs::InternFileId;
use ide_db::RootDatabase;
use ide_db::helpers::visit_file_defs;
use indexmap::IndexSet;
use itertools::Itertools;
use lang::loc::SyntaxLocFileExt;
use lang::{Semantics, item_specs};
use syntax::files::{FilePosition, InFile, InFileExt};
use syntax::{AstNode, TextRange, ast};
use vfs::FileId;

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Annotation {
    pub range: TextRange,
    pub kind: AnnotationKind,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum AnnotationKind {
    // Runnable(Runnable),
    HasSpecs {
        pos: FilePosition,
        item_specs: Option<Vec<NavigationTarget>>,
    },
    // HasReferences { pos: FilePosition, data: Option<Vec<FileRange>> },
}

pub struct AnnotationConfig {
    pub annotate_fun_specs: bool,
    pub location: AnnotationLocation,
}

pub enum AnnotationLocation {
    AboveName,
    AboveWholeItem,
}

pub(crate) fn annotations(
    db: &RootDatabase,
    config: &AnnotationConfig,
    file_id: FileId,
) -> Vec<Annotation> {
    let mut annotations: IndexSet<Annotation> = IndexSet::default();

    visit_file_defs(&Semantics::new(db, file_id), file_id, &mut |module_item| {
        match module_item {
            ast::NamedElement::Fun(fun) if config.annotate_fun_specs => {
                let fun = fun.in_file(file_id);
                if let Some(_) =
                    item_specs::get_item_specs_for_items_in_file(db, file_id.intern(db)).get(&fun.loc())
                {
                    let (annotation_range, target_pos) = make_ranges(config, fun);
                    annotations.insert(Annotation {
                        range: annotation_range,
                        kind: AnnotationKind::HasSpecs {
                            pos: target_pos,
                            item_specs: None,
                        },
                    });
                }
            }
            _ => {
                return None;
            }
        };
        Some(())
    });

    annotations
        .into_iter()
        .sorted_by_key(|a| (a.range.start(), a.range.end()))
        .collect()
}

pub(crate) fn resolve_annotation(db: &RootDatabase, mut annotation: Annotation) -> Annotation {
    match annotation.kind {
        AnnotationKind::HasSpecs {
            pos,
            item_specs: ref mut item_spec,
        } => {
            *item_spec = goto_specification(db, pos).map(|range| range.info);
        }
    };
    annotation
}

fn make_ranges(
    config: &AnnotationConfig,
    item: InFile<impl Into<ast::NamedElement>>,
) -> (TextRange, FilePosition) {
    let (file_id, item) = item.unpack();
    let item = item.into();

    let focus_range = item.name().map(|it| it.syntax().text_range());
    let full_range = item.syntax().text_range();

    let cmd_target = focus_range.unwrap_or(full_range);
    let annotation_range = match config.location {
        AnnotationLocation::AboveName => cmd_target,
        AnnotationLocation::AboveWholeItem => full_range,
    };
    let target_pos = FilePosition {
        file_id,
        offset: cmd_target.start(),
    };
    (annotation_range, target_pos)
}
