# rust-navigator

Rust-Navigator is a faster, less accurate alternative to Rust-Analyzer. Where Rust-Analyzer relies 
on using the compiler to get perfectly accurate diagnostics and code actions, Rust-Navigator sticks
to source code scanning and prioritizes implementing features in a way that is always fast, instead
of always accurate.

Rust-Navigator reuses some parts of Rust-Analyzer, namely the parser, which remains perfectly suitable
as a rust LSP parser.

## Features

- Syntax errors
- Insert `(pub) mod` in a parent module from the child module
- Limited go-to-definition for types and functions