#!/bin/bash

cargo clean

cross build --release --target x86_64-unknown-linux-gnu
if [ $? -ne 0 ]; then
    echo "Compilation for linux x64 failed..."
    return 1
fi

cross build --release --target x86_64-pc-windows-gnu --verbose
if [ $? -ne 0 ]; then
    echo "Compilation for windows failed..."
    return 1
fi

cross build --release --target aarch64-unknown-linux-gnu
if [ $? -ne 0 ]; then
    echo "Compilation for linux arm64 failed..."
    return 1
fi