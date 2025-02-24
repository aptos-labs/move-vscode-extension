use crate::RootDatabase;
use base_db::change::FileChange;
use ra_salsa::{Database, Durability};

impl RootDatabase {
    pub fn request_cancellation(&mut self) {
        let _p = tracing::info_span!("RootDatabase::request_cancellation").entered();
        self.synthetic_write(Durability::LOW);
    }

    pub fn apply_change(&mut self, change: FileChange) {
        let _p = tracing::info_span!("RootDatabase::apply_change").entered();
        self.request_cancellation();

        tracing::trace!("apply_change {:?}", change);

        // if let Some(roots) = &change.roots {
        //     let mut local_roots = FxHashSet::default();
        //     let mut library_roots = FxHashSet::default();
        //     for (idx, root) in roots.iter().enumerate() {
        //         let root_id = SourceRootId(idx as u32);
        //         if root.is_library {
        //             library_roots.insert(root_id);
        //         } else {
        //             local_roots.insert(root_id);
        //         }
        //     }
        //     self.set_local_roots_with_durability(Arc::new(local_roots), Durability::HIGH);
        //     self.set_library_roots_with_durability(Arc::new(library_roots), Durability::HIGH);
        // }

        change.apply(self);
    }
}
