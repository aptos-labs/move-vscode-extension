// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

//! Simple logger that logs either to stderr or to a file, using `tracing_subscriber`
//! filter syntax and `tracing_appender` for non blocking output.

use crate::tracing::hprof;
use std::env;
use tracing_subscriber::{EnvFilter, Layer, Registry, fmt::MakeWriter, layer::SubscriberExt};
use tracing_tree::HierarchicalLayer;

#[derive(Debug)]
pub struct LoggingConfig<T> {
    pub writer: T,
    pub default_level: tracing::Level,
}

impl<T> LoggingConfig<T>
where
    T: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    pub fn try_init(self) -> anyhow::Result<()> {
        let LoggingConfig { writer, default_level } = self;

        let default_filter = EnvFilter::builder()
            .with_env_var("RA_LOG")
            .with_default_directive(default_level.into())
            .from_env()
            .unwrap_or_else(|err| {
                tracing::error!("invalid directives in RA_LOG: {}", err);
                Default::default()
            })
            .add_directive("salsa::=error".parse()?);

        let profiler = env::var("APT_PROFILER").ok().is_some();
        if profiler {
            hprof::span_tree().aggregate(true).enable();
            return Ok(());
        }

        let deferred_spans = !env::var("APT_LOG_SHOW_EMPTY_SPANS").ok().is_some();
        let subscriber = Registry::default().with(
            HierarchicalLayer::new(2)
                .with_ansi(false)
                .with_indent_lines(true)
                .with_deferred_spans(deferred_spans)
                .with_writer(writer)
                .with_filter(default_filter),
        );

        tracing::subscriber::set_global_default(subscriber)?;

        Ok(())
    }
}
