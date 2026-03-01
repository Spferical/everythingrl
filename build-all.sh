#!/usr/bin/env bash
set -xeo pipefail

# ugly hack for cross to work on nixos
if [ -e /nix/store ]; then
    export NIX_STORE=/nix/store
fi

# Build Linux (Cross-compiled on CentOS for max compatibility)
cross build --target=x86_64-unknown-linux-gnu --release
target_dir=$(cargo metadata --format-version=1 | jq -r .target_directory)
rm -rf dist_linux
mkdir -p dist_linux
cp "${target_dir}/x86_64-unknown-linux-gnu/release/everythingrl" dist_linux/

# Build Windows
nix build .#windows
rm -rf dist_windows
mkdir -p dist_windows
cp result/bin/everythingrl.exe dist_windows/
touch dist_windows/*

# Build WASM
nix build .#wasm
rsync -a result/dist/ dist/
touch dist/*

# Clean up symlink
rm result
