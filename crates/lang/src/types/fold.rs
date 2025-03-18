use crate::types::ty::TypeFolder;

pub trait TypeFoldable<T> {
    fn deep_fold_with(self, folder: impl TypeFolder) -> T;
}
