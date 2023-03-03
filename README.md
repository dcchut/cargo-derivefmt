# cargo-derivefmt

A tool for ordering derives in Rust code.

## Purpose

`cargo-derivefmt` ensures that your derives are written in alphabetical order.

```rust
// Before: pure chaos.
#[derive(Debug, PartialEq, Ord, Clone, Copy)]
struct S;

// After: blissful order.
#[derive(Clone, Copy, Debug, Ord, PartialEq)]
struct S;
```

That's it!  That's all this does.

## Install guide

```shell
cargo install --locked --git https://github.com/dcchut/derivefmt --bin cargo-derivefmt
```

## Quickstart guide

```shell
cargo derivefmt 
```

### Formatting a single file.

Pass a single path to format a single file:

```shell
cargo derivefmt path/to/src.rs
```

### Formatting multiple files.

Pass multiple paths to format multiple files:

```shell
cargo derivefmt path/to/src.rs /path/to/another.rs
```

### Formatting folders

Passing a folder formats all `.rs` files contained within it and its subfolders:

```shell
cargo derivefmt /path/to/folder/
```

### Formatting from STDIN

Pass `-` as the path to format input from stdin:

```shell
echo "#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]" | derivefmt -
```

## Roadmap

- Better identification of files to format (particularly in the context of a cargo project).
- Better handling of comments.

