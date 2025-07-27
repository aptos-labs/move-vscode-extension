// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ast;

impl ast::ChooseExpr {
    pub fn quant_binding_ident_pat(&self) -> Option<ast::IdentPat> {
        self.quant_binding().and_then(|it| it.ident_pat())
    }
}
