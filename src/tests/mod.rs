use textwrap::dedent;

use crate::modify_source;

#[test]
fn test_derive_ordering() {
    let mut source = dedent(
        r#"
        #[derive(Eq, Ord, PartialOrd, Copy, Clone, Debug, PartialEq, Hash)]
        struct Wrapped<T>(T);
    "#,
    );
    modify_source(&mut source).unwrap();
    insta::assert_snapshot!(source);
}

#[test]
fn test_derive_ordering_qualified() {
    let mut source = dedent(
        r#"
        #[derive ( PartialEq, Copy, PartialOrd, std :: fmt :: Debug, Hash, Eq, Ord )]
        struct SillyBugger;
    "#,
    );
    modify_source(&mut source).unwrap();
    insta::assert_snapshot!(source);
}

#[test]
fn test_multiline_derives() {
    let mut source = dedent(
        r#"
        #[derive(
            core::cmp::PartialEq,
            core::clone::Clone,
            core::marker::Copy,
            core::fmt::Debug,
            core::hash::Hash,
            core::default::Default,
            core::cmp::Eq,
            core::cmp::Ord,
            core::cmp::PartialOrd,
        )]
        struct Core;

        #[derive(
            std::fmt::Debug,
            std::clone::Clone,
            std::marker::Copy,
            std::cmp::PartialEq,
            std::default::Default,
            std::cmp::Eq,
            std::hash::Hash,
            std::cmp::Ord,
            std::cmp::PartialOrd,
        )]
        struct Std;

        "#,
    );
    modify_source(&mut source).unwrap();
    insta::assert_snapshot!(source);
}
