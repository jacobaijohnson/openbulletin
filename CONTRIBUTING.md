# Contributing

- The engine crates (`ob-*`) must build and test headless. The GUI shell
  stays thin; features land in the engine and the CLI first.
- Every feature PR answers one question in its description: does a church
  secretary need this before v1.0? See docs/NEVER.md for the standing no's.
- Imposition and file-format changes require new unit tests in the same PR.
- Run `cargo fmt`, `cargo clippy`, and `cargo test` before pushing; CI
  runs the same on Windows, macOS, and Linux.
