#!/bin/bash
cargo update --precise 1.6.0 -p base64ct@1.8.0 > /dev/null 2>&1

echo "Building main project..."
if cargo build > /dev/null 2>&1; then
    echo "✓ Main build: PASS"
else
    echo "✗ Main build: FAIL"
    exit 1
fi

echo "Building Solana program..."
cd program
if cargo build-sbf > /dev/null 2>&1; then
    echo "✓ Program build: PASS"
else
    echo "✗ Program build: FAIL"
    exit 1
fi
cd ..

echo "All builds completed successfully!"
