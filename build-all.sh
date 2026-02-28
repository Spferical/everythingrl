#!/usr/bin/env bash
set -xeo pipefail

# Build Linux (default)
nix build .#default
rm -rf dist_linux
mkdir -p dist_linux
cp result/bin/everythingrl dist_linux/

# Build Windows
nix build .#windows
rm -rf dist_windows
mkdir -p dist_windows
cp result/bin/everythingrl.exe dist_windows/

# Build WASM
nix build .#wasm
rm -rf dist_wasm
mkdir -p dist_wasm
cp -r result/dist/* dist_wasm/

# Clean up symlink
rm result
