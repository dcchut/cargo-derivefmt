use parser::SyntaxKind::{COMMA, WHITESPACE};
use syntax::SyntaxToken;

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

pub fn sorted_groups(groups: Vec<Vec<SyntaxToken>>) -> Vec<Vec<SyntaxToken>> {
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
    parts.sort_by_key(|tokens| ident(tokens).map(String::from));

    // Re-insert the fixed points
    for (idx, component) in fixed_points {
        parts.insert(idx, component);
    }

    parts
}
