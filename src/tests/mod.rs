use textwrap::dedent;

use crate::modify_source;

#[test]
fn test_derive_ordering() {
    let mut source = dedent(
        r#"
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
        #[derive(Clone, Copy, std :: fmt :: Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        struct SillyBugger;
    "#,
    );
    modify_source(&mut source).unwrap();
    insta::assert_snapshot!(source);
}
