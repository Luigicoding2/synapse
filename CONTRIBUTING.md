# Contributing to Synapse

Thank you for your interest in contributing to the **Synapse Systems Trilogy**! We welcome bug reports, feature suggestions, performance optimizations, and pull requests.

## Architecture Overview

Synapse is divided into three modular Rust crates and a cross-platform Go binding layer:
- `synapse-store`: LSM-Tree based Key-Value storage engine.
- `synapse-search`: In-memory fuzzy search, prefix trie autocomplete, and TF-IDF index.
- `synapse-bridge`: C-ABI interop boundary exposing store and search symbols.
- `bindings/`: Go bindings supporting pure-Go Windows DLL loading and Unix CGO static linking.

## Development Setup

### Prerequisites
- **Rust** 1.75+ (`rustup default stable`)
- **Go** 1.21+

### Running Tests

Run all Rust unit and integration tests across the workspace:
```bash
cargo test --workspace
```

Build release binaries and static libraries:
```bash
cargo build --release
```

Run Go integration tests and latency benchmarks:
```bash
cd bindings
go test -v .
go test -bench=. -benchmem .
```

## Pull Request Guidelines

1. Ensure all workspace tests pass (`cargo test --workspace`).
2. Include unit tests for any new algorithms or API endpoints.
3. Keep code formatting clean with `cargo fmt`.
4. Document public APIs with doc comments (`///`).

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
