macro_rules! test {
    ($contents: expr) => {
        let mut source = textwrap::dedent($contents);
        crate::modify_source(&mut source);
        insta::assert_snapshot!(source);
    };
}

#[test]
fn test_derive_missing_comma() {
    test!(
        r#"
        #[derive(Debug, Clone, Default Hash)]
        struct S;
    "#
    );
}

#[test]
fn test_derive_double_comma() {
    test!(
        r#"
        #[derive(Debug, Clone, Default,, Apples, Hash)]
        struct T;
    "#
    );
}

#[test]
fn test_derive_ordering() {
    test!(
        r#"
        #[derive(Eq, Ord, PartialOrd, Copy, Clone, Debug, PartialEq, Hash)]
        struct Wrapped<T>(T);
    "#
    );
}

#[test]
fn test_derive_ordering_qualified() {
    test!(
        r#"
        #[derive ( PartialEq, Copy, PartialOrd, std :: fmt :: Debug, Hash, Eq, Ord )]
        struct SillyBugger;
    "#
    );
}

#[test]
fn test_multiline_derives() {
    test!(
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

        "#
    );
}

#[test]
fn test_derive_comments() {
    test!(
        r#"
        #[derive(
        /* ---------- Some really important comment that just had to go inside the derive --------- */
        Debug,
        Clone, // And what about this?
        Eq, PartialEq,
        )]
        struct Foo {
            a: i32,
            b: T,
        }
        "#
    );
}
