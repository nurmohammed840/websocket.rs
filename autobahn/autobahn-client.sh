#!/usr/bin/env bash
set -euo pipefail
set -x
SOURCE_DIR=$(readlink -f "${BASH_SOURCE[0]}")
SOURCE_DIR=$(dirname "$SOURCE_DIR")
cd "${SOURCE_DIR}/.."

CONTAINER_NAME=fuzzingserver
function cleanup() {
    docker container stop "${CONTAINER_NAME}"
}
trap cleanup TERM EXIT

docker run -d --rm \
    -v "${PWD}/autobahn:/autobahn" \
    -p 9001:9001 \
    --init \
    --name "${CONTAINER_NAME}" \
    crossbario/autobahn-testsuite \
    wstest -m fuzzingserver -s 'autobahn/fuzzingserver.json'

sleep 3

cargo run --release --example autobahn