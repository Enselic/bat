#!/usr/bin/env bash
set -o errexit -o nounset -o pipefail

RUSTDOCFLAGS='-Z unstable-options --output-format json' cargo +nightly doc --no-deps

jq  '.index | .[] | select(.crate_id == 0) | select(.visibility == "public") | .name' target/doc/bat.json | sort > doc/public-api-symbols.txt
