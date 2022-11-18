#!/bin/bash

cargo clean
cargo build --release
cargo build --target x86_64-unknown-linux-musl --release
cargo build --release --target=x86_64-pc-windows-gnu

mkdir -p bin

cp target/x86_64-unknown-linux-musl/release/edit_config bin
cp target/x86_64-pc-windows-gnu/release/edit_config.exe bin

chmod +x bin/*
