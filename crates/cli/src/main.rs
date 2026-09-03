// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! terminus command-line entry point. Runs a scenario and produces a
//! self-describing output directory (per-node PCAPNG, matching
//! dissectors, visualether.toml, metrics, config snapshot).

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "terminus", version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run a scenario TOML → out/<scenario>/run-<unix-secs>/
    Run {
        scenario: PathBuf,
        #[arg(long, default_value = "out")]
        out: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().cmd {
        Cmd::Run { scenario, out } => {
            let loaded = terminus_cli::config::load(&scenario)?;
            // Wall clock is fine HERE (outside the simulation): the
            // run-id names the directory; the contents stay
            // byte-deterministic regardless.
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock before 1970")
                .as_secs();
            let run_dir = out
                .join(&loaded.file.scenario.name)
                .join(format!("run-{stamp}"));
            drop(loaded);
            terminus_cli::assemble::run_scenario(&scenario, &run_dir)?;
            println!("{}", run_dir.display());
            Ok(())
        }
    }
}
