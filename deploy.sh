#!/bin/bash

# export CC="aarch64-unknown-linux-gnu-gcc"
export PATH=$PATH:$PWD/toolchain/bin/
export CC="aarch64-none-linux-gnu-gcc"
cargo run --package robot --bin robot --release --target aarch64-unknown-linux-gnu
