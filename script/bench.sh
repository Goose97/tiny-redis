#!/bin/bash
set -e
cargo run &
_server_pid=$!
echo $_server_pid
cargo run --bin redis_benchmark
kill _server_pid 2> /dev/null