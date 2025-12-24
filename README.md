# match-string

A Rust library for simple and flexible string pattern matching.
[![Crates.io](https://img.shields.io/crates/v/match-string.svg)](https://crates.io/crates/match-string)
[![Documentation](https://docs.rs/match-string/badge.svg)](https://docs.rs/match-string)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

## Examples

Simple matches using built-in tokens:

```rust
matches!("Hello, World!"    => ALPHABETIC, ", ", ALPHABETIC);
matches!("123 456"          => NUM, " ", NUM);
matches!("[12,34,56]"     => "[", NUM[","]+, "]");
matches!("foobarfoofoobar"  => ("foo" / "bar")+);
```

Capturing matched values:

```rust
let name: Dest<String> = Dest::new();
let greeting = matches!("Hello, Alice!" => "Hello, ", name@ALPHABETIC, "!");

let arrays: Dest<Vec<usize>> = Dest::new();
let numbers = matches!("[1,2,3]" => "[", (arrays@NUM)[","]+, "]");
```

Custom tokens:

```rust
const VOWELS: Token<char, String> = Token {
    /* Check each character */
    predicate: |ch| "aeiouAEIOU".contains(*ch),
    /* Convert Vec<char> to String */
    parser: |v| v.into_iter().collect(),
    /* Require at least one match */
    at_least: 1,
    /* Skip leading whitespace */
    skip_leading: Some(|ch: &char| ch.is_whitespace()), 
};
```