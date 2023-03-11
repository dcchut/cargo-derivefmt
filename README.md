# cargo-derivefmt

A tool for ordering derives in Rust code.

## Motivation

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

## Installation

This package is currently implemented using rust-analyzer internals, so cannot be published on crates.io.

```shell
cargo install --locked --git https://github.com/dcchut/cargo-derivefmt --bin cargo-derivefmt
```

## Usage

### Formatting the current crate

```shell
cargo derivefmt 
```

### Formatting a different crate

```shell
cargo derivefmt --manifest-path /path/to/Cargo.toml
```

### Formatting a single file

```shell
cargo derivefmt --file path/to/src.rs
```

### Formatting multiple files


```shell
cargo derivefmt --file path/to/src.rs --file /path/to/another.rs
```

### Formatting folders

Passing a folder formats all `.rs` files contained within it and its subfolders:

```shell
cargo derivefmt --file /path/to/folder/
```

## Roadmap

- Better identification of files to format (particularly in the context of a cargo project).
- Better handling of comments.

