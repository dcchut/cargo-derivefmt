use std::{collections::HashSet, path::PathBuf};

use anyhow::Result;
use cargo_files_core::{get_target_files, get_targets};
use clap::Parser;
use rayon::prelude::*;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
// Cargo passes "derivefmt" to cargo-derivefmt, so add a hidden argument to capture that.
#[command(
    arg(clap::Arg::new("dummy")
    .value_parser(["derivefmt"])
    .required(false)
    .hide(true))
)]
struct Args {
    /// Path to file or folder to format.  Can be specified multiple times.
    #[arg(short, long = "file", conflicts_with = "manifest_path")]
    files: Vec<PathBuf>,
    /// Path to Cargo.toml
    #[arg(long, conflicts_with = "files")]
    manifest_path: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut files: Vec<PathBuf> = Vec::new();
    if args.files.is_empty() {
        let targets = get_targets(args.manifest_path.as_deref())?;
        for target in targets {
            files.extend(get_target_files(&target)?);
        }
    } else {
        let mut resolved_files = HashSet::with_capacity(args.files.len());
        for file in args.files {
            if file.is_dir() {
                let glob = file.join("**").join("*.rs");
                for entry in glob::glob(&glob.to_string_lossy())? {
                    let path = entry?;
                    resolved_files.insert(path);
                }
            } else {
                resolved_files.insert(file);
            }
        }
        files.extend(resolved_files);
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
