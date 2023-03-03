mod build;
mod modify;
mod parse;
mod sort;
#[cfg(test)]
mod tests;

use std::{
    collections::HashSet,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::Parser;
use rayon::prelude::*;

use crate::modify::modify_source;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The path to the files and folders that should be formatted.
    #[clap(name = "file")]
    file: Vec<PathBuf>,
}

#[derive(Debug, Eq, Hash, PartialEq)]
enum Target {
    /// Input from stdin
    Stdin,
    /// A path to a source file.
    Path(PathBuf),
}

impl Target {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        Ok(if path.to_string_lossy() == "-" {
            Self::Stdin
        } else {
            Self::Path(path.canonicalize()?)
        })
    }

    pub fn to_string(&self) -> Result<String> {
        match self {
            Self::Stdin => {
                let mut buf = String::new();
                std::io::stdin()
                    .read_to_string(&mut buf)
                    .with_context(|| "failed to read from stdin")?;
                Ok(buf)
            }
            Self::Path(path) => std::fs::read_to_string(path)
                .with_context(|| format!("failed to read source file {}", path.display())),
        }
    }

    pub fn write(&self, source: String) -> Result<()> {
        match self {
            Self::Stdin => {
                println!("{source}");
            }
            Self::Path(path) => {
                std::fs::write(path, source)
                    .with_context(|| format!("failed to write {}", path.display()))?;
            }
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    let mut files = Args::parse().file;

    // If no files are specified we assume the user wants to format CWD.
    if files.is_empty() {
        files.push(std::env::current_dir()?);
    }

    let mut resolved_files = HashSet::with_capacity(files.len());
    for file in files {
        if file.is_dir() {
            let glob = file.join("**").join("*.rs");
            for entry in glob::glob(&glob.to_string_lossy())? {
                let path = entry?;
                resolved_files.insert(Target::new(path)?);
            }
        } else {
            resolved_files.insert(Target::new(file)?);
        }
    }

    resolved_files
        .par_iter()
        .map(|target| {
            let mut source = target.to_string()?;
            modify_source(&mut source)?;
            target.write(source)?;
            Ok(())
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(())
}
