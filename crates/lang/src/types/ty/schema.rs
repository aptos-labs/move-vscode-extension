use crate::loc::{SyntaxLoc, SyntaxLocFileExt};
use crate::types::fold::{TypeFoldable, TypeFolder, TypeVisitor};
use crate::types::has_type_params_ext::GenericItemExt;
use crate::types::substitution::Substitution;
use crate::types::ty::Ty;
use base_db::SourceDatabase;
use syntax::ast;
use syntax::files::InFile;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TySchema {
    pub schema_loc: SyntaxLoc,
    pub substitution: Substitution,
    pub type_args: Vec<Ty>,
}

impl TySchema {
    pub fn new(item: InFile<ast::Schema>) -> Self {
        TySchema {
            schema_loc: item.loc(),
            substitution: item.ty_type_params_subst(),
            type_args: item
                .ty_type_params()
                .into_iter()
                .map(|it| Ty::TypeParam(it))
                .collect(),
        }
    }

    pub fn schema(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::Schema>> {
        self.schema_loc.to_ast::<ast::Schema>(db)
    }
}

impl TypeFoldable<TySchema> for TySchema {
    fn deep_fold_with(self, folder: impl TypeFolder) -> TySchema {
        TySchema {
            schema_loc: self.schema_loc,
            substitution: self.substitution.deep_fold_with(folder.clone()),
            type_args: folder.fold_tys(self.type_args),
        }
    }

    fn deep_visit_with(&self, visitor: impl TypeVisitor) -> bool {
        self.substitution.deep_visit_with(visitor.clone()) || visitor.visit_tys(&self.type_args)
    }
}
