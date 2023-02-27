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
    SyntaxKind::{COMMA, IDENT, L_PAREN, R_PAREN, WHITESPACE},
};
use rayon::prelude::*;
use syntax::{
    algo::diff,
    ast,
    ast::{make::token, AttrKind},
    ted,
    ted::{Element, Position},
    AstNode,
    NodeOrToken::Token,
    SyntaxNode, SyntaxToken, SyntaxTreeBuilder,
};
use text_edit::TextEdit;

#[derive(Clone, Debug)]
struct Component<'a> {
    ident: Option<&'a str>,
    tokens: &'a [SyntaxToken],
}

impl<'a> Component<'a> {
    // <LEFT WHITESPACE>std:: fmt:: Debug<RIGHT WHITESPACE>
    // ^ sep
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
            .last();

        Self { ident, tokens }
    }
}

fn reorder_components(components: &mut Vec<Component<'_>>) {
    let fixed_points = components
        .iter()
        .enumerate()
        .filter(|(_, component)| component.ident.is_none())
        .collect::<Vec<_>>();

    let mut parts = components.to_vec();
    parts.retain(|x| x.ident.is_some());
    parts.sort_by_key(|c| c.ident);

    for (i, comp) in fixed_points {
        parts.insert(i, comp.clone());
    }

    *components = parts;
    // We may want some sort of grouping behaviour, e.g.;
    // Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord
    //                     <----------->  <------------->
    // For now, just sort alphabetically.
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

        let mut builder = SyntaxTreeBuilder::default();
        builder.start_node(SyntaxKind::TOKEN_TREE);

        let mut separators: Vec<Vec<SyntaxToken>> = vec![];
        let mut initial_separator = None;
        let mut groups = vec![];
        let mut acc = vec![];
        for token in &tokens[1..(tokens.len() - 1)] {
            if token.kind() == COMMA {
                assert!(!acc.is_empty());

                // Is there any left whitespace?
                let left_whitespace_idx = acc.iter().position(|x: &SyntaxToken| x.kind() != WHITESPACE && x.kind() != COMMA)
                    .unwrap_or(0);
                let right_whitespace_idx = acc.iter().rposition(|x: &SyntaxToken| x.kind() != WHITESPACE && x.kind() != COMMA)
                    .unwrap_or(acc.len()-1);

                // 0..0 left_whitespace_idx..=right_whitespace_idx right_whitespace_idx..acc.len();

                let l = &acc[0..left_whitespace_idx];
                let m = &acc[left_whitespace_idx..=right_whitespace_idx];
                let r = &acc[(right_whitespace_idx+1)..];

                if !l.is_empty() {
                    // TODO: what if separators is empty?
                    if separators.is_empty() {
                        initial_separator = Some(l.to_vec());
                    } else {
                        let q = separators.len() - 1;
                        separators[q].extend_from_slice(l);
                    }
                }

                let mut sep = vec![token.clone()];
                sep.extend_from_slice(r);
                separators.push(sep);
                //         let left_whitespace = tokens.split(|t| t.kind() != WHITESPACE && t.kind() != COMMA)
                //             .next().unwrap();
                //         let right_whitespace = tokens.rsplit(|t| t.kind() != WHITESPACE && t.kind() != COMMA)
                //             .next().unwrap();

                groups.push(acc[left_whitespace_idx..=right_whitespace_idx].to_vec());
                acc.clear();
                // std::mem::take(&mut acc));
            } else {
                acc.push(token.clone());
            }
        }
        if !acc.is_empty() {
            groups.push(acc);
        }

        // Now build components
        let components: Vec<_> = groups
            .iter()
            .map(|group| Component::new(group.as_slice()))
            .collect();

        let mut sorted_components = components.clone();
        reorder_components(&mut sorted_components);

        if let Some(separators) = initial_separator {
            for token in separators {
                builder.token(token.kind(), token.text());
            }
        }

        let mut sep_iter = separators.into_iter();
        for component in sorted_components {
            for token in component.tokens {
                builder.token(token.kind(), token.text());
            }
            if let Some(sep) = sep_iter.next() {
                for token in sep {
                    builder.token(token.kind(), token.text());
                }
            }
        }
        for sep in sep_iter
        {
            for token in sep {
                builder.token(token.kind(), token.text());
            }
        }
        builder.finish_node();

        let parse = builder.finish().syntax_node().clone_for_update();

        println!("BEFORE:\n{}", tree);

        ted::replace_all(
            tokens[1].clone().syntax_element()..=tokens[tokens.len() - 2].clone().syntax_element(),
            vec![parse.syntax_element()],
        );

        println!("AFTER:\n{}", tree);
        //
        // dbg!(parse);
        //
        // // L_PAREN@9..10 "(",
        // //     IDENT@10..15 "Clone",
        // //     COMMA@15..16 ",",
        // //     WHITESPACE@16..17 " ",
        // //     IDENT@17..21 "Copy",
        // //     COMMA@21..22 ",",
        // //     WHITESPACE@22..23 " ",
        // //     IDENT@23..26 "std",
        // //     WHITESPACE@26..27 " ",
        // //     COLON@27..28 ":",
        // //     COLON@28..29 ":",
        // //     WHITESPACE@29..30 " ",
        // //     IDENT@30..33 "fmt",
        // //     WHITESPACE@33..34 " ",
        // //     COLON@34..35 ":",
        // //     COLON@35..36 ":",
        // //     WHITESPACE@36..37 " ",
        // //     IDENT@37..42 "Debug",
        // //     COMMA@42..43 ",",
        // //     WHITESPACE@43..44 " ",
        // //     IDENT@44..46 "Eq",
        // //     COMMA@46..47 ",",
        // //     WHITESPACE@47..48 " ",
        // //     IDENT@48..52 "Hash",
        // //     COMMA@52..53 ",",
        // //     WHITESPACE@53..54 " ",
        // //     IDENT@54..57 "Ord",
        // //     COMMA@57..58 ",",
        // //     WHITESPACE@58..59 " ",
        // //     IDENT@59..68 "PartialEq",
        // //     COMMA@68..69 ",",
        // //     WHITESPACE@69..70 " ",
        // //     IDENT@70..80 "PartialOrd",
        // //     R_PAREN@80..81 ")",
        // //  WHITESPACE@401..406 "\n    ",
        //
        // dbg!(&tokens);
        //
        // // We should have _at least_ ( and ).
        // if tokens.len() < 2
        //     || tokens[0].kind() != L_PAREN
        //     || tokens.last().unwrap().kind() != R_PAREN
        // {
        //     continue;
        // }
        //
        // let components: Vec<_> = tokens[1..tokens.len() - 1]
        //     .split_inclusive(|token| token.kind() == COMMA)
        //     .filter_map(Component::new)
        //     .collect();
        //
        // let mut sorted_components = components.clone();
        // reorder_components(&mut sorted_components);
        //
        // // for (before, after) in components.into_iter().zip(sorted_components) {
        // //     let t: SyntaxToken = todo!();
        // //
        // //
        // //     before.tokens
        // // }
        //
        //
        // ted::remove_all(
        //     tokens[0].clone().syntax_element()..=tokens[tokens.len() - 1].clone().syntax_element(),
        // );
        //
        // for (i, component) in sorted_components.into_iter().enumerate() {
        //     // Remove whitespace from the start and end.
        //     let tt: Vec<_> = component.tokens.to_vec();
        //     let vv: Vec<_> = tt
        //         .into_iter()
        //         .map(|c| c.syntax_element())
        //         .chain(if i > 0 {
        //             Some(Token(token(COMMA)))
        //         } else {
        //             None
        //         })
        //         .collect();
        //
        //     ted::insert_all(Position::first_child_of(tree.syntax()), vv);
        // }
        //
        // ted::insert(Position::first_child_of(tree.syntax()), token(L_PAREN));
        // ted::insert(Position::last_child_of(tree.syntax()), token(R_PAREN));

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
