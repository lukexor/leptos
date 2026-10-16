#!/usr/bin/env bash
# Starts the release server in the background and prints its pid.
cd "$(dirname "$0")" || exit
LEPTOS_SITE_ROOT=target/site LEPTOS_SITE_PKG_DIR=pkg LEPTOS_OUTPUT_NAME=dispose-repro \
LEPTOS_SITE_ADDR=127.0.0.1:3000 RUST_BACKTRACE=1 \
  setsid ./target/release/dispose-repro > server.log 2>&1 &
echo $!
