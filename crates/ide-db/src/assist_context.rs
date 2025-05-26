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
        let mut f = Some(f);
        self.add_impl(id, label.into(), target, &mut |it| f.take().unwrap()(it))
    }

    fn add_impl(
        &mut self,
        id: AssistId,
        label: String,
        target: TextRange,
        f: &mut dyn FnMut(&mut SourceChangeBuilder),
    ) -> Option<()> {
        // if !self.is_allowed(&id) {
        //     return None;
        // }

        // let mut command = None;
        let source_change = if self.resolve.should_resolve(&id) {
            let mut builder = SourceChangeBuilder::new(self.file);
            f(&mut builder);
            // command = builder.command.take();
            Some(builder.finish())
        } else {
            None
        };

        let label = Label::new(label);
        self.buf.push(Assist {
            id,
            label,
            // group,
            target,
            source_change,
            command: None,
        });
        Some(())
    }
}
