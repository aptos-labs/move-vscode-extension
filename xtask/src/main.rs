mod codegen;
mod install;

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::env;
use std::path::PathBuf;
use xshell::Shell;

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Codegen,
    Install {
        #[clap(long)]
        server: bool,
        #[clap(long)]
        client: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let sh = Shell::new()?;
    match cli.command {
        Command::Codegen => codegen::generate(),
        Command::Install { client, server } => {
            if client {
                install::install_client(&sh).context("install client")?;
            }
            if server {
                install::install_server(&sh).context("install server")?;
            }
        }
    }

    Ok(())
}

/// Returns the path to the root directory of `rust-analyzer` project.
fn project_root() -> PathBuf {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned());
    PathBuf::from(dir).parent().unwrap().to_owned()
}
