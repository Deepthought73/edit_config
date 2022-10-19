#!/bin/bash

cargo build --release
cargo build --release --target=x86_64-pc-windows-gnu

mkdir -p bin

cp target/release/edit_config bin
cp target/x86_64-pc-windows-gnu/release/edit_config.exe bin

chmod +x bin/*
