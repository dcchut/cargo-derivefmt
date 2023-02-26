use crate::modify_source;
use textwrap::dedent;

#[test]
fn test_derive_ordering() {
    let mut source = dedent(
        r#"
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        struct Wrapped<T>(T);
    "#,
    );

    modify_source(&mut source).unwrap();
    insta::assert_snapshot!(source);
}
