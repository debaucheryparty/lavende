#!/bin/bash
set -e

# Go up to src/go and build the Rust library in release mode
cd ..
cargo build --release

# Copy the generated header and static library to the Go module folder
cp lavende.h lavende/
cp ../../target/release/liblavende_go.a lavende/

echo "Rust C-API built and copied to lavende/"
