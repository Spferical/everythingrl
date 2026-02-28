#!/usr/bin/env bash
# Filter out -fuse-ld=lld because it's broken in this environment
args=()
for arg in "$@"; do
    if [[ "$arg" != "-fuse-ld=lld" ]]; then
        args+=("$arg")
    fi
done
exec cc "${args[@]}"
