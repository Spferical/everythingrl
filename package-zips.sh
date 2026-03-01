#!/usr/bin/env bash
set -e

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <version>"
    exit 1;
fi
version=$1

rm -f ./everythingrl-*-"${version}".zip
(cd dist_windows && zip -r "../everythingrl-windows-${version}.zip" *)
(cd dist_linux && zip -r "../everythingrl-linux-${version}.zip" *)
(cd dist_wasm && zip -r "../everythingrl-web.zip" *)
(cd dist_windows && zip -r "../everythingrl-windows-${version}.zip" *)
