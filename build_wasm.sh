#!/usr/bin/env bash
set -eo pipefail

./wasm-bindgen-macroquad
rm -rf dist/assets/
cp -r assets/ dist/assets/
root=$(cargo metadata --format-version=1 | jq -r .resolve.root)
project_name=$(cargo metadata --format-version=1 \
               | jq -r ".packages[] | select(.id==\"${root}\") | .name")
wasm-opt -Os dist/${project_name}_bg.wasm -o dist/${project_name}_bg.wasm
