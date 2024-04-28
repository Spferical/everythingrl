#!/usr/bin/env bash
set -eo pipefail
cargo build --release --target wasm32-unknown-unknown
target_dir=$(cargo metadata --format-version=1 | jq -r .target_directory)
root=$(cargo metadata --format-version=1 | jq -r .resolve.root)
project_name=$(cargo metadata --format-version=1 \
               | jq -r ".packages[] | select(.id==\"${root}\") | .name")
cp -f "${target_dir}/wasm32-unknown-unknown/release/${project_name}.wasm" dist/
rm -rf dist/assets/
cp -r assets/ dist/assets/
