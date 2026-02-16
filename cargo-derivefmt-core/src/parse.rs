use parser::SyntaxKind::{COMMA, WHITESPACE};
use syntax::SyntaxToken;

pub struct ParsedDerive {
    /// The separator after each group.
    pub separators: Vec<Vec<SyntaxToken>>,

    /// Optional separator before the first group.
    pub leading_separator: Option<Vec<SyntaxToken>>,

    /// An entry in the derive corresponding to a trait.
    pub groups: Vec<Vec<SyntaxToken>>,
}

impl ParsedDerive {
    pub fn parse(tokens: &[SyntaxToken]) -> ParsedDerive {
        let mut separators: Vec<Vec<SyntaxToken>> = Vec::new();
        let mut groups = Vec::new();
        let mut leading_separator = None;

        let mut acc = Vec::new();
        for token in tokens.iter().map(Some).chain([None]) {
            if let Some(token) = token
                && token.kind() != COMMA
            {
                acc.push(token.clone());
                continue;
            }

            if acc.is_empty() {
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

        ParsedDerive {
            separators,
            leading_separator,
            groups,
        }
    }
}
