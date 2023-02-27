#[cfg(test)]
mod tests;

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use clap::Parser;
use parser::SyntaxKind::{COMMA, L_PAREN, R_PAREN, WHITESPACE};
use rayon::prelude::*;
use syntax::{
    algo::diff,
    ast,
    ast::{make::token, AttrKind},
    ted,
    ted::{Element, Position},
    AstNode,
    NodeOrToken::Token,
    SyntaxToken,
};
use text_edit::TextEdit;

#[derive(Clone, Debug)]
struct Component<'a> {
    ident: &'a str,
    tokens: &'a [SyntaxToken],
}

impl<'a> Component<'a> {
    pub fn new(tokens: &'a [SyntaxToken]) -> Self {
        let ident = tokens
            .iter()
            .filter_map(|t| {
                if t.kind() != WHITESPACE && t.kind() != COMMA {
                    Some(t.text())
                } else {
                    None
                }
            })
            .last()
            .unwrap();

        Self { tokens, ident }
    }
}

fn reorder_components(components: &mut [Component<'_>]) {
    // We may want some sort of grouping behaviour, e.g.;
    // Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord
    //                     <----------->  <------------->
    // For now, just sort alphabetically.
    components.sort_by_key(|c| std::cmp::Reverse(c.ident));
}

// TODO: clean up this abomination
pub fn modify_source(source: &mut String) -> Result<()> {
    let parse = syntax::SourceFile::parse(source);
    let file: syntax::SourceFile = parse.tree();

    let mut edits = Vec::new();

    let mut tokens = Vec::new();
    'item: for item in file.syntax().descendants().filter_map(ast::Attr::cast) {
        if item.kind() != AttrKind::Outer {
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

        // We should have _at least_ ( and ).
        if tokens.len() < 2
            || tokens[0].kind() != L_PAREN
            || tokens.last().unwrap().kind() != R_PAREN
        {
            continue;
        }

        let components: Vec<_> = tokens[1..tokens.len() - 1]
            .split_inclusive(|token| token.kind() == COMMA)
            .map(Component::new)
            .collect();

        let mut sorted_components = components.clone();
        reorder_components(&mut sorted_components);

        ted::remove_all(
            tokens[0].clone().syntax_element()..=tokens[tokens.len() - 1].clone().syntax_element(),
        );

        for (i, component) in sorted_components.into_iter().enumerate() {
            // Remove whitespace from the start and end.
            let tt: Vec<_> = component.tokens.to_vec();

            let l = tt.iter().position(|x| x.kind() != WHITESPACE).unwrap();
            let r = tt
                .iter()
                .rposition(|x| x.kind() != COMMA && x.kind() != WHITESPACE)
                .unwrap();

            let tt: Vec<_> = tt[l..=r].to_vec();
            let vv: Vec<_> = tt
                .into_iter()
                .map(|c| c.syntax_element())
                .chain(if i > 0 {
                    Some(Token(token(COMMA)))
                } else {
                    None
                })
                .collect();

            ted::insert_all(Position::first_child_of(tree.syntax()), vv);
        }

        ted::insert(Position::first_child_of(tree.syntax()), token(L_PAREN));
        ted::insert(Position::last_child_of(tree.syntax()), token(R_PAREN));

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
