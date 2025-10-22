#!/bin/bash

cargo clean
rm Cargo.lock

cargo build

cargo update --precise 1.6.0 -p base64ct@1.8.0

cd program
cargo build-sbf
cd ..
