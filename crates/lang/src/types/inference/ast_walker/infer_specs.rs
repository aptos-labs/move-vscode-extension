use crate::nameres::scope::ScopeEntryExt;
use crate::types::expectation::Expected;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::substitution::ApplySubstitution;
use crate::types::ty::Ty;
use crate::types::ty::schema::TySchema;
use std::iter::zip;
use syntax::ast;
use syntax::ast::node_ext::spec_predicate_stmt::SpecPredicateKind;
use syntax::files::{InFile, InFileExt};

impl<'a, 'db> TypeAstWalker<'a, 'db> {
    pub(super) fn process_predicate_stmt(&mut self, predicate: &ast::SpecPredicateStmt) -> Option<()> {
        let expr = predicate.expr()?;
        let kind = predicate.kind()?;
        match kind {
            SpecPredicateKind::Assume
            | SpecPredicateKind::Assert
            | SpecPredicateKind::Requires
            | SpecPredicateKind::Ensures
            | SpecPredicateKind::Decreases => {
                self.infer_expr_coerceable_to(&expr, Ty::Bool);
            }
            SpecPredicateKind::Modifies => {
                self.infer_expr(&expr, Expected::NoValue);
            }
        }
        Some(())
    }

    pub(super) fn process_aborts_if_stmt(&mut self, aborts_if_stmt: &ast::AbortsIfStmt) -> Option<()> {
        let expr = aborts_if_stmt.expr()?;
        self.infer_expr_coerceable_to(&expr, Ty::Bool);
        if let Some(aborts_with) = aborts_if_stmt.aborts_if_with() {
            let with_expr = aborts_with.expr()?;
            self.infer_expr_coerceable_to(&with_expr, Ty::Num);
        }
        Some(())
    }

    pub(super) fn process_include_schema(&mut self, include_schema: &ast::IncludeSchema) -> Option<()> {
        let include_expr = include_schema.include_expr()?;
        for schema_lit in include_expr.schema_lits() {
            self.process_schema_lit(&schema_lit);
        }
        Some(())
    }

    pub(super) fn process_schema_lit(&mut self, schema_lit: &ast::SchemaLit) -> Option<()> {
        let path = schema_lit.path()?;
        let item = self
            .ctx
            .resolve_path_cached(path.clone(), None)
            .and_then(|it| it.cast_into::<ast::Schema>());
        let schema = match item {
            Some(schema) => schema,
            None => {
                // not schema, just infer field exprs and be done
                let field_exprs = schema_lit
                    .fields()
                    .into_iter()
                    .filter_map(|it| it.expr())
                    .collect::<Vec<_>>();
                for field_expr in field_exprs {
                    self.infer_expr(&field_expr, Expected::NoValue);
                }
                return None;
            }
        };
        let ty_schema = self
            .ctx
            .instantiate_path(path.into(), schema.clone())
            .into_ty_schema()?;
        for schema_lit_field in schema_lit.fields() {
            let expected_field_ty = self
                .get_schema_field_ty(&ty_schema, &schema_lit_field)
                .unwrap_or(Ty::Unknown);
            if let Some(expr) = schema_lit_field.expr() {
                self.infer_expr_coerceable_to(&expr, expected_field_ty);
            }
        }
        Some(())
    }

    fn get_schema_field_ty(
        &mut self,
        ty_schema: &TySchema,
        schema_lit_field: &ast::SchemaLitField,
    ) -> Option<Ty> {
        let field_name = schema_lit_field.field_name()?;
        let schema_fields = ty_schema.schema(self.ctx.db)?.flat_map(|it| it.schema_fields());
        let schema_field = schema_fields
            .iter()
            .find(|it| it.value.name().map(|n| n.as_string()) == Some(field_name.clone()))?
            .to_owned();
        let type_ = schema_field.and_then(|it| it.type_())?;
        Some(
            self.ctx
                .ty_lowering()
                .lower_type(type_)
                .substitute(&ty_schema.substitution),
        )
    }

    pub(super) fn infer_quant_expr(&mut self, quant_expr: &ast::QuantExpr) -> Option<Ty> {
        for quant_binding in quant_expr.quant_bindings() {
            if let Some(ident_pat) = quant_binding.ident_pat() {
                let ty = self.infer_quant_binding_ty(&quant_binding).unwrap_or(Ty::Unknown);
                self.ctx.pat_types.insert(ident_pat.into(), ty);
            }
        }
        if let Some(where_expr) = quant_expr.where_expr().and_then(|it| it.expr()) {
            self.infer_expr_coerceable_to(&where_expr, Ty::Bool);
        }
        self.infer_quant_expr_inner_ty(quant_expr);
        Some(Ty::Bool)
    }

    fn infer_quant_binding_ty(&mut self, quant_binding: &ast::QuantBinding) -> Option<Ty> {
        let ty = if quant_binding.in_token().is_some() {
            let range_expr = quant_binding.expr()?;
            let seq_ty = self.infer_expr(&range_expr, Expected::NoValue).into_ty_seq()?;
            seq_ty.item().refine_for_specs(true)
        } else {
            let type_ = quant_binding.type_()?;
            self.ctx.ty_lowering().lower_type(type_.in_file(self.ctx.file_id))
        };
        Some(ty)
    }

    fn infer_quant_expr_inner_ty(&mut self, quant_expr: &ast::QuantExpr) -> Option<()> {
        match quant_expr {
            ast::QuantExpr::ForallExpr(forall_expr) => {
                let expr = forall_expr.expr()?;
                self.infer_expr_coerceable_to(&expr, Ty::Bool);
            }
            ast::QuantExpr::ExistsExpr(exists_expr) => {
                let expr = exists_expr.expr()?;
                self.infer_expr_coerceable_to(&expr, Ty::Bool);
            }
            ast::QuantExpr::ChooseExpr(_) => (),
        }
        Some(())
    }

    pub(crate) fn collect_item_spec_signature_bindings(
        &mut self,
        item_spec: &ast::ItemSpec,
        item: InFile<ast::Item>,
    ) {
        let (file_id, item) = item.unpack();
        match item {
            ast::Item::Fun(fun) => {
                let fun_params = fun.to_any_fun().params_as_bindings();
                let param_ident_pats = item_spec.param_ident_pats();
                for (param_ident_pat, fun_param) in zip(param_ident_pats, fun_params.clone()) {
                    if let Some(param_ident_pat) = param_ident_pat {
                        let entry = if param_ident_pat.name().is_some()
                            && fun_param.name().is_some()
                            && param_ident_pat.name().unwrap().as_string()
                                == fun_param.name().unwrap().as_string()
                        {
                            fun_param.in_file(file_id).to_entry()
                        } else {
                            None
                        };
                        self.ctx.resolved_ident_pats.insert(param_ident_pat, entry);
                    }
                }
            }
            _ => (),
        }
    }
}
