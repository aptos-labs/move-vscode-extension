mod diagnostics;

use crate::cli::diagnostics::Diagnostics;
use clap::{Args, Parser, Subcommand};
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
}
