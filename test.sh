# Build the project
./build.sh

SBF_OUT_DIR=$(pwd)/target/deploy cargo nextest run
