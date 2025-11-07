#!/bin/bash
set -e

# Default to test-program
TEST_CLIENT="${1:-test-program}"
TEST_FILTER="${2:-}"
STACK_SIZE=16777216

echo "Building project..."
./build.sh
cp target/deploy/jito_bls_ncn_program.so integration_tests/tests/bins

echo "Running tests with client: $TEST_CLIENT"
if [ -n "$TEST_FILTER" ]; then
    echo "Test filter: $TEST_FILTER"
fi

case "$TEST_CLIENT" in
    "surfpool")
        echo "Using Surfpool client"
        RUST_MIN_STACK=$STACK_SIZE SBF_OUT_DIR=$(pwd)/target/deploy cargo nextest run --no-default-features --features surfpool --test-threads=1 $TEST_FILTER
        ;;
    "test-program")
        echo "Using SolanaTestProgram client (default)"
        RUST_MIN_STACK=$STACK_SIZE SBF_OUT_DIR=$(pwd)/target/deploy cargo nextest run --features test-program $TEST_FILTER
        ;;
    "default")
        echo "Using default features"
        RUST_MIN_STACK=$STACK_SIZE SBF_OUT_DIR=$(pwd)/target/deploy cargo nextest run $TEST_FILTER
        ;;
    *)
        echo "Error: Unknown test client '$TEST_CLIENT'"
        echo "Usage: ./test.sh [test-program|surfpool|default] [test-filter]"
        echo "  test-program (default) - Use SolanaTestProgram client"
        echo "  surfpool               - Use Surfpool client"
        echo "  default                - Use default cargo features"
        echo ""
        echo "Examples:"
        echo "  ./test.sh                                    # Run all tests with test-program"
        echo "  ./test.sh test-program test_multiple_votes  # Run specific test"
        echo "  ./test.sh surfpool program::vote::tests     # Run test module with surfpool"
        exit 1
        ;;
esac

echo "Tests completed successfully!"
