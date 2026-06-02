#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail

VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')

./build.sh

NAME=log_searcher-$VERSION

rm -fr $NAME
rm -f $NAME.zip
mkdir $NAME/

cp install.sh log_searcher.initd log_searcher.service  README.md target/x86_64-unknown-linux-musl/release/log_searcher $NAME/

zip -r $NAME.zip $NAME/

echo "Release ready: $NAME.zip"
