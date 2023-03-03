use parser::SyntaxKind;
use syntax::{SyntaxNode, SyntaxToken, SyntaxTreeBuilder};

/// Builds the new [`SyntaxNode`] for this derive.
pub fn build_derive_node(
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
