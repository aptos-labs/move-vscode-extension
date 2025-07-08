// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use aptos_language_server::tracing::LoggingConfig;
use tracing::Level;
use tracing_subscriber::fmt::writer::BoxMakeWriter;

pub fn init_tracing_for_test() {
    let _ = LoggingConfig {
        writer: BoxMakeWriter::new(std::io::stdout),
        default_level: Level::DEBUG,
    }
    .try_init();
}
