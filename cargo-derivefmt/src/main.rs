use std::path::PathBuf;

use anyhow::Result;
use cargo_files_core::{get_target_files, get_targets};
use clap::Parser;
use rayon::prelude::*;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
// Cargo passes "files" to cargo-files, so add a hidden argument to capture that.
#[command(
    arg(clap::Arg::new("dummy")
    .value_parser(["derivefmt"])
    .required(false)
    .hide(true))
)]
struct Args {
    /// Path to Cargo.toml
    #[arg(long)]
    manifest_path: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut files = Vec::new();
    let targets = get_targets(args.manifest_path.as_deref())?;
    dbg!(targets);
    panic!();
    for target in targets {
        files.extend(get_target_files(&target)?);
    }

    files
        .into_par_iter()
        .map(|path| {
            let mut source = std::fs::read_to_string(&path)?;
            cargo_derivefmt_core::modify_source(&mut source);
            std::fs::write(&path, source)?;
            Ok(())
        })
        .collect::<Result<_>>()?;

    Ok(())
}
