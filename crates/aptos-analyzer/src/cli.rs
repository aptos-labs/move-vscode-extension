mod check;

use crate::cli::check::Check;
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
    Check(Check),
}
