use parser::SyntaxKind::{COMMA, L_PAREN, R_PAREN, WHITESPACE};
use std::process::Command;
use syntax::algo::diff;
use syntax::ast::make::token;
use syntax::ast::AttrKind;
use syntax::ted::{Element, Position};
use syntax::NodeOrToken::Token;
use syntax::{ast, ted, AstNode, SyntaxToken};
use text_edit::TextEdit;

#[derive(Clone, Copy, std :: fmt :: Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct SillyBugger;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct SqueezeIt;

fn main() {
    let src = std::fs::read_to_string("src/main.rs").unwrap();

    let parse = syntax::SourceFile::parse(&src);
    let file: syntax::SourceFile = parse.tree();

    let mut edits = Vec::new();

    'item: for item in file.syntax().descendants().filter_map(ast::Attr::cast) {
        if item.kind() != AttrKind::Outer {
            continue;
        }

        if item.simple_name() != Some("derive".into()) {
            continue;
        }

        let Some(_tree) = item.token_tree() else { continue; };
        let tree = _tree.clone_for_update();

        let mut tokens = Vec::new();

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

        let components: Vec<_> = tokens[1..tokens.len() - 1]
            .split_inclusive(|token| token.kind() == COMMA)
            .map(Component::new)
            .collect();

        let mut sorted_components = components.clone();
        sorted_components.sort_by_key(|c| std::cmp::Reverse(c.ident));

        ted::remove_all(
            tokens[0].clone().syntax_element()..=tokens[tokens.len() - 1].clone().syntax_element(),
        );

        for (i, component) in sorted_components.into_iter().enumerate() {
            // Remove whitespace from the start and end.
            let tt: Vec<_> = component.tokens.into_iter().cloned().collect();

            let l = tt.iter().position(|x| x.kind() != WHITESPACE).unwrap();
            let r = tt
                .iter()
                .rposition(|x| x.kind() != COMMA && x.kind() != WHITESPACE)
                .unwrap();

            let tt: Vec<_> = tt[l..=r].into_iter().cloned().collect();
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
    let mut src = file.to_string();
    text_edit.apply(&mut src);

    // Write the output to disk & format it.
    std::fs::write("src/main.rs.swp", src).expect("failed to write output");

    // TODO: confirm rustfmt exists
    Command::new("rustfmt")
        .args(["src/main.rs.swp"])
        .output()
        .expect("failed to run rustfmt");

    // Now replace .swp with the real thing
    std::fs::copy("src/main.rs.swp", "src/main.rs").expect("failed to copy output");
    std::fs::remove_file("src/main.rs.swp").expect("failed to delete swpfile");
}
