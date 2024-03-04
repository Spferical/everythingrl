#!/usr/bin/env bash
nix build '.#wasm'
cp -f result/bin/seven-drl-2024.wasm dist/
rm -rf dist/assets/
cp -r assets/ dist/assets/
