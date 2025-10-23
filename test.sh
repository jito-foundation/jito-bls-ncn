#!/bin/bash

set -e

# Default to test-program
TEST_CLIENT="${1:-test-program}"

echo "Building project..."
./build.sh

cp target/deploy/jito_bls_ncn_program.so integration_tests/tests/bins

echo "Running tests with client: $TEST_CLIENT"

case "$TEST_CLIENT" in
    "surfpool")
        echo "Using Surfpool client"
        SBF_OUT_DIR=$(pwd)/target/deploy cargo nextest run --no-default-features --features surfpool
        ;;
    "test-program")
        echo "Using SolanaTestProgram client (default)"
        SBF_OUT_DIR=$(pwd)/target/deploy cargo nextest run --features test-program
        ;;
    "default")
        echo "Using default features"
        SBF_OUT_DIR=$(pwd)/target/deploy cargo nextest run
        ;;
    *)
        echo "Error: Unknown test client '$TEST_CLIENT'"
        echo "Usage: ./test.sh [test-program|surfpool|default]"
        echo "  test-program (default) - Use SolanaTestProgram client"
        echo "  surfpool               - Use Surfpool client"
        echo "  default                - Use default cargo features"
        exit 1
        ;;
esac

echo "Tests completed successfully!"
