#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail

rustup target add x86_64-unknown-linux-musl

cargo build --release --target x86_64-unknown-linux-musl

echo "success"
