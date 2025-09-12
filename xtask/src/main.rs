// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

#![allow(dead_code)]

mod codegen;
mod copyright;
mod dist;
mod install;
mod testgen;

use clap::{Parser, Subcommand};
use std::env;
use std::path::PathBuf;
use xshell::{Shell, cmd};

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Codegen,
    Testgen,
    Install {
        #[clap(long)]
        server: bool,
        #[clap(long)]
        client: bool,
        #[clap(long)]
        offline: bool,
    },
    Dist {
        #[clap(long)]
        client_patch_version: Option<String>,
    },
    Copyright,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Codegen => codegen::generate(),
        Command::Testgen => testgen::generate(),
        Command::Install { client, server, offline } => {
            install::install(client, server, offline)?;
        }
        Command::Dist { client_patch_version } => dist::dist(client_patch_version)?,
        Command::Copyright => copyright::enforce(),
    }

    Ok(())
}

/// Returns the path to the root directory of `rust-analyzer` project.
fn project_root() -> PathBuf {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned());
    PathBuf::from(dir).parent().unwrap().to_owned()
}

fn date_iso(sh: &Shell) -> anyhow::Result<String> {
    let res = cmd!(sh, "date -u +%Y-%m-%d").read()?;
    Ok(res)
}

fn is_release_tag(tag: &str) -> bool {
    tag.len() == "2020-02-24".len() && tag.starts_with(|c: char| c.is_ascii_digit())
}
