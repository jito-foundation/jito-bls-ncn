#!/bin/bash
set -e

# Check if running in GitHub CI mode
GITHUB_MODE=false
if [ "$1" = "github" ]; then
    GITHUB_MODE=true
fi

if [ "$GITHUB_MODE" = true ]; then
    echo "🔍 Checking package sorting..."
    cargo sort --workspace --check

    echo "✨ Checking code formatting..."
    cargo fmt --all --check
else
    echo "🔍 Sorting packages..."
    cargo sort --workspace

    echo "✨ Formatting code..."
    cargo fmt --all
fi

echo "📎 Running clippy..."
cargo clippy --all-features --all-targets --tests -- -D warnings

echo "🔨 Building project..."
./build.sh

echo "🧪 Running tests..."
./test.sh

if [ "$GITHUB_MODE" = false ]; then
    echo "🚀 Installing CLI..."
    cargo install --path cli
fi

echo "✅ All CI checks passed!"
