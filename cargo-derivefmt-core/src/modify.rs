use ide_db::{syntax_helpers::tree_diff::diff, text_edit::TextEdit};
pub use parser::Edition;
use syntax::{
    AstNode,
    ast::{Attr, AttrKind},
    syntax_editor::{Element, SyntaxEditor},
};

use crate::{build::build_derive_node, parse::ParsedDerive, sort::sorted_groups};

pub fn modify_source(source: &mut String, edition: Edition) {
    let parse = syntax::SourceFile::parse(source, edition);
    let file: syntax::SourceFile = parse.tree();
    let (editor, root) = SyntaxEditor::new(file.syntax().clone());

    let mut tokens = Vec::new();

    'item: for item in root.descendants().filter_map(Attr::cast) {
        if item.kind() != AttrKind::Outer {
            continue;
        }

        let Some((name, tree)) = item.as_simple_call() else {
            continue;
        };
        if name != "derive" {
            continue;
        }

        tokens.clear();
        for node_or_token in tree.token_trees_and_tokens() {
            let Some(token) = node_or_token.into_token() else {
                continue 'item;
            };
            tokens.push(token);
        }
        // Nothing to sort in an empty derive.
        if tokens.len() <= 2 {
            continue;
        }

        let derive = ParsedDerive::parse(&tokens[1..(tokens.len() - 1)]);
        let sorted_groups = sorted_groups(derive.groups);
        let node = build_derive_node(derive.leading_separator, derive.separators, sorted_groups);

        editor.replace_all(
            tokens[1].clone().syntax_element()..=tokens[tokens.len() - 2].clone().syntax_element(),
            vec![node.syntax_element()],
        );
    }

    let edit = editor.finish();
    let mut builder = TextEdit::builder();
    diff(edit.old_root(), edit.new_root()).into_text_edit(&mut builder);
    builder.finish().apply(source);
}
