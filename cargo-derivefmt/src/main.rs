use std::{collections::HashSet, path::PathBuf};

use anyhow::Result;
use cargo_files_core::{Edition, get_target_files, get_targets};
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

    let mut files: Vec<(Option<Edition>, PathBuf)> = Vec::new();
    if args.files.is_empty() {
        let targets = get_targets(args.manifest_path.as_deref())?;
        for target in targets {
            files.extend(
                get_target_files(&target)?
                    .into_iter()
                    .map(|f| (Some(target.edition), f)),
            );
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
        // In the absence of any indication from Cargo, we likely should accept an --edition
        // flag for this use-case, however I'm lazy so I'm not going to do that (yet).
        files.extend(resolved_files.into_iter().map(|f| (None, f)));
    }

    files
        .into_par_iter()
        .map(|(edition, path)| {
            let edition = match edition {
                Some(Edition::E2021) => cargo_derivefmt_core::Edition::Edition2021,
                Some(Edition::E2018) => cargo_derivefmt_core::Edition::Edition2018,
                Some(Edition::E2015) => cargo_derivefmt_core::Edition::Edition2015,
                Some(Edition::E2024) => cargo_derivefmt_core::Edition::Edition2024,
                _ => cargo_derivefmt_core::Edition::CURRENT,
            };

            let mut source = std::fs::read_to_string(&path)?;
            cargo_derivefmt_core::modify_source(&mut source, edition);
            std::fs::write(&path, source)?;
            Ok(())
        })
        .collect::<Result<_>>()
}
