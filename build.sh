#!/bin/bash

cargo build --release
cargo build --release --target=x86_64-pc-windows-gnu

cp target/release/edit_config .
cp target/x86_64-pc-windows-gnu/release/edit_config.exe .
