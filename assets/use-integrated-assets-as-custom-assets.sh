#!/usr/bin/env bash
set -o errexit -o nounset -o pipefail

# Run this script from bat git repo root

mkdir -p ~/.cache/bat
echo "Created dir ~/.cache/bat"

# version extraction copied from CICD.yml
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)
echo -e "---\nbat_version: ${version}" > ~/.cache/bat/metadata.yaml
echo "Wrote ~/.cache/bat/metadata.yaml"

cp assets/syntaxes.bin ~/.cache/bat/syntaxes.bin
echo "Wrote ~/.cache/bat/syntaxes.bin"

cp assets/themes.bin ~/.cache/bat/themes.bin
echo "Wrote ~/.cache/bat/themes.bin"
