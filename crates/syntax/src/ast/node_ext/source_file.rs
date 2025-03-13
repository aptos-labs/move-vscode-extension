use crate::ast;

impl ast::SourceFile {
    pub fn all_modules(&self) -> impl Iterator<Item = ast::Module> {
        self.modules()
            .chain(self.address_defs().flat_map(|ad| ad.modules()))
    }
}
