// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use std::fmt;
use std::sync::Arc;
use stdx::itertools::Itertools;

#[derive(Debug)]
pub enum ConfigErrorInner {
    Json { config_key: String, error: serde_json::Error },
    // Toml { config_key: String, error: toml::de::Error },
    // ParseError { reason: String },
}

#[derive(Clone, Debug, Default)]
pub struct ConfigErrors(pub Vec<Arc<ConfigErrorInner>>);

impl ConfigErrors {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for ConfigErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let errors = self.0.iter().format_with("\n", |inner, f| {
            match &**inner {
                ConfigErrorInner::Json { config_key: key, error: e } => {
                    f(key)?;
                    f(&": ")?;
                    f(e)
                } // ConfigErrorInner::Toml { config_key: key, error: e } => {
                  //     f(key)?;
                  //     f(&": ")?;
                  //     f(e)
                  // }
                  // ConfigErrorInner::ParseError { reason } => f(reason),
            }?;
            f(&";")
        });
        write!(
            f,
            "invalid config value{}:\n{}",
            if self.0.len() == 1 { "" } else { "s" },
            errors
        )
    }
}

impl std::error::Error for ConfigErrors {}
