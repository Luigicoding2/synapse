#!/usr/bin/env bash
set -e

echo "⚡ Building Synapse Release Libraries..."
cargo build --release

echo "🦀 Running Rust Workspace Tests..."
cargo test --workspace

echo "🔷 Running Go Bindings Tests & Benchmarks..."
(cd bindings && go test -v . && go test -bench="." -benchmem .)

echo "🚀 Running Demos..."
cargo run -p rust-demo
(cd examples/go_demo && go run main.go)

echo "✨ All builds and verification tests passed successfully!"
