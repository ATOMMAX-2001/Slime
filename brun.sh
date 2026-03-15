#!/usr/bin/env bash

cleanup() {
    cd ..
}

trap cleanup INT

set -e

try_run() {
    echo "Running build and tests..."

    cd web
    maturin develop
    cd ..

    cd slimeweb
    uv run python -Xgil=0 -m test.test
}

if ! try_run; then
    echo "Build Failed"
    cd ..
fi
