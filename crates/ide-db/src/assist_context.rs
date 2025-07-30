// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::assists::{Assist, AssistId, AssistResolveStrategy};
use crate::label::Label;
use crate::source_change::SourceChangeBuilder;
use syntax::TextRange;
use vfs::FileId;

pub struct Assists {
    file: FileId,
    resolve: AssistResolveStrategy,
    buf: Vec<Assist>,
    // allowed: Option<Vec<AssistKind>>,
}

impl Assists {
    pub fn new(file_id: FileId, resolve: AssistResolveStrategy) -> Assists {
        Assists {
            resolve,
            file: file_id,
            buf: Vec::new(),
            // allowed: ctx.config.allowed.clone(),
        }
    }

    pub fn assists(self) -> Vec<Assist> {
        self.buf
    }

    pub fn add(
        &mut self,
        id: AssistId,
        label: impl Into<String>,
        target: TextRange,
        f: impl FnOnce(&mut SourceChangeBuilder),
    ) -> Option<()> {
        let label = label.into();
        let source_change = if self.resolve.should_resolve(&id) {
            let mut builder = SourceChangeBuilder::new(self.file);
            f(&mut builder);
            Some(builder.finish())
        } else {
            None
        };
        self.buf.push(Assist {
            id,
            label: Label::new(label),
            target,
            source_change,
            command: None,
        });
        Some(())
    }
}
