#!/usr/bin/env bash
set -ex

# Install lld in the container
yum install -y llvm-toolset-7.0-lld || apt-get update && apt-get install -y lld || true
