#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail

VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')

./build.sh

rm -fr package
mkdir package

cp install.sh  log_searcher.initd  log_searcher.service  README.md target/x86_64-unknown-linux-musl/release/log_searcher package/

zip -r log_searcher-$VERSION.zip package/

echo "Release ready: log_searcher-$VERSION.zip"
