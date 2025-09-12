// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

mod bench;
mod diagnostics;

use crate::cli::bench::Bench;
use crate::cli::diagnostics::Diagnostics;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct CliArgs {
    #[arg(long)]
    pub version: bool,

    /// Log to the specified file instead of stderr.
    #[arg(long)]
    pub log_file: Option<PathBuf>,

    #[clap(subcommand)]
    pub subcommand: Option<AptosAnalyzerCmd>,
}

#[derive(Debug, Subcommand)]
pub enum AptosAnalyzerCmd {
    LspServer,
    Diagnostics(Diagnostics),
    Bench(Bench),
}
