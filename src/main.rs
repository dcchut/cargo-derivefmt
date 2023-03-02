#[cfg(test)]
mod tests;

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use clap::Parser;
use parser::{
    SyntaxKind,
    SyntaxKind::{COMMA, WHITESPACE},
};
use rayon::prelude::*;
use syntax::{
    algo::diff, ast, ted, ted::Element, AstNode, SyntaxNode, SyntaxToken, SyntaxTreeBuilder,
};
use text_edit::TextEdit;

fn ident(tokens: &[SyntaxToken]) -> Option<&str> {
    tokens
        .iter()
        .filter_map(|t| {
            if t.kind() != WHITESPACE && t.kind() != COMMA {
                Some(t.text())
            } else {
                None
            }
        })
        .last()
}

fn sorted_groups(groups: Vec<Vec<SyntaxToken>>) -> Vec<Vec<SyntaxToken>> {
    // Identify any fixed points - these are components that don't have an ident
    // which we don't want to move in the derive.
    let mut fixed_points = Vec::new();
    let mut parts = Vec::with_capacity(groups.len());

    for (idx, group) in groups.into_iter().enumerate() {
        if ident(&group).is_some() {
            parts.push(group);
        } else {
            fixed_points.push((idx, group));
        }
    }
    parts.sort_by_key(|tokens| ident(&tokens).map(String::from));

    // Re-insert the fixed points
    for (idx, component) in fixed_points {
        parts.insert(idx, component);
    }

    parts
}

struct ParsedDerive {
    /// The separator after each group.
    separators: Vec<Vec<SyntaxToken>>,

    /// Optional separator before the first group.
    leading_separator: Option<Vec<SyntaxToken>>,

    /// An entry in the derive corresponding to a trait.
    groups: Vec<Vec<SyntaxToken>>,
}

impl ParsedDerive {
    pub fn parse(tokens: &[SyntaxToken]) -> ParsedDerive {
        let mut separators: Vec<Vec<SyntaxToken>> = Vec::new();
        let mut groups = Vec::new();
        let mut leading_separator = None;

        let mut acc = Vec::new();
        for token in tokens.iter().map(Some).chain([None]) {
            if let Some(token) = token {
                if token.kind() != COMMA {
                    acc.push(token.clone());
                    continue;
                }
            }

            if token.is_none() && acc.is_empty() {
                // If the accumulator is empty then we don't have a trailing separator.
                continue;
            }

            // acc here will typically look like " Clone", or perhaps "\n    Clone".
            // Identify the "Clone" span.  Note that we also have more complex examples
            // like " std:: fmt:: Debug " where the span should be "std:: fmt:: Debug".
            let ident_start = acc
                .iter()
                .position(|t| t.kind() != WHITESPACE && t.kind() != COMMA)
                .unwrap_or(0);
            let ident_end = acc
                .iter()
                .rposition(|t| t.kind() != WHITESPACE && t.kind() != COMMA)
                .unwrap_or(acc.len() - 1);

            if ident_start > 0 {
                if separators.is_empty() {
                    // First token has a leading separator
                    leading_separator = Some(acc[0..ident_start].to_vec());
                } else {
                    separators
                        .last_mut()
                        .unwrap()
                        .extend_from_slice(&acc[0..ident_start]);
                }
            }

            let mut separator = Vec::new();
            separator.extend(token.cloned());
            separator.extend_from_slice(&acc[(ident_end + 1)..]);
            separators.push(separator);

            groups.push(acc[ident_start..=ident_end].to_vec());
            acc.clear();
        }

        dbg!(&acc);

        ParsedDerive {
            separators,
            leading_separator,
            groups,
        }
    }
}

/// Builds the new [`SyntaxNode`] for this derive.
fn build_derive_node(
    leading_separator: Option<Vec<SyntaxToken>>,
    separators: Vec<Vec<SyntaxToken>>,
    groups: Vec<Vec<SyntaxToken>>,
) -> SyntaxNode {
    let mut builder = SyntaxTreeBuilder::default();
    builder.start_node(SyntaxKind::TOKEN_TREE);

    fn extend<I: IntoIterator<Item = SyntaxToken>>(builder: &mut SyntaxTreeBuilder, iter: I) {
        for token in iter {
            builder.token(token.kind(), token.text());
        }
    }

    if let Some(separators) = leading_separator {
        extend(&mut builder, separators);
    }

    let mut separators = separators.into_iter();
    for group in groups {
        extend(&mut builder, group);
        if let Some(comma) = separators.next() {
            extend(&mut builder, comma);
        }
    }

    for comma in separators {
        extend(&mut builder, comma);
    }
    builder.finish_node();
    builder.finish().syntax_node()
}

// TODO: clean up this abomination
pub fn modify_source(source: &mut String) -> Result<()> {
    let parse = syntax::SourceFile::parse(source);
    let file: syntax::SourceFile = parse.tree();

    let mut edits = Vec::new();
    let mut tokens = Vec::new();
    'item: for item in file.syntax().descendants().filter_map(ast::Attr::cast) {
        if item.kind() != ast::AttrKind::Outer {
            continue;
        }

        if item.simple_name() != Some("derive".into()) {
            continue;
        }

        let Some(_tree) = item.token_tree() else { continue; };
        let tree = _tree.clone_for_update();

        tokens.clear();
        for node_or_token in tree.token_trees_and_tokens() {
            if let Some(token) = node_or_token.into_token() {
                tokens.push(token);
            } else {
                continue 'item;
            }
        }

        let derive = ParsedDerive::parse(&tokens[1..(tokens.len() - 1)]);
        let sorted_groups = sorted_groups(derive.groups);
        let parse = build_derive_node(derive.leading_separator, derive.separators, sorted_groups)
            .clone_for_update();

        ted::replace_all(
            tokens[1].clone().syntax_element()..=tokens[tokens.len() - 2].clone().syntax_element(),
            vec![parse.syntax_element()],
        );

        let mut builder = TextEdit::builder();
        let d = diff(_tree.syntax(), tree.syntax());
        d.into_text_edit(&mut builder);
        let text_edit = builder.finish();
        edits.push(text_edit);
    }

    // Merge all text edits together
    let text_edit = edits.into_iter().fold(TextEdit::default(), |mut u, v| {
        u.union(v).unwrap();
        u
    });
    text_edit.apply(source);
    Ok(())
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The path to the files and folders that should be formatted.
    #[clap(name = "file")]
    file: Vec<PathBuf>,
}

fn reorder_derives_in_file<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    let mut source = std::fs::read_to_string(path).with_context(|| "failed to read source file")?;
    modify_source(&mut source)?;

    std::fs::write(path, source).with_context(|| format!("failed to write {}", path.display()))
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct Target {
    /// A path to a source file.
    path: PathBuf,
}

impl Target {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().canonicalize()?;
        Ok(Target { path })
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
        if !file.exists() {
            anyhow::bail!("file {} does not exist", file.display());
        }

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
        .map(|file| reorder_derives_in_file(&file.path))
        .collect::<Result<Vec<_>>>()?;

    // Now run rustfmt on all the files we modified.
    Command::new("rustfmt")
        .args(resolved_files.iter().map(|file| &file.path))
        .output()
        .with_context(|| "failed to format output files")?;

    Ok(())
}
