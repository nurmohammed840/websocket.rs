#!/usr/bin/env bash
# Author michael <themichaeleden@gmail.com>
set -euo pipefail
set -x
SOURCE_DIR=$(readlink -f "${BASH_SOURCE[0]}")
SOURCE_DIR=$(dirname "$SOURCE_DIR")
cd "${SOURCE_DIR}/.."

function cleanup() {
    kill -9 ${WSSERVER_PID}
}
trap cleanup TERM EXIT

cargo run --release --example echo -- 127.0.0.1:9002 & WSSERVER_PID=$!
sleep 5

docker run --rm \
    -v "${PWD}/autobahn:/autobahn" \
    --network host \
    crossbario/autobahn-testsuite \
    wstest -m fuzzingclient -s 'autobahn/fuzzingclient.json'
