#!/usr/bin/env bash
set -eo pipefail

./wasm-bindgen-macroquad
rm -rf dist/assets/
cp -r assets/ dist/assets/
