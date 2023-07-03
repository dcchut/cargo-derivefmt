use syntax::{
    algo::diff,
    ast::{Attr, AttrKind},
    ted,
    ted::Element,
    AstNode,
};
use text_edit::TextEdit;

use crate::{build::build_derive_node, parse::ParsedDerive, sort::sorted_groups};

pub fn modify_source(source: &mut String) {
    let parse = syntax::SourceFile::parse(source);
    let file: syntax::SourceFile = parse.tree();

    let mut edits = Vec::new();
    let mut tokens = Vec::new();

    'item: for item in file.syntax().descendants().filter_map(Attr::cast) {
        if item.kind() != AttrKind::Outer {
            continue;
        }

        if item.simple_name() != Some("derive".into()) {
            continue;
        }

        let Some(_tree) = item.token_tree() else {
            continue;
        };
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
}
