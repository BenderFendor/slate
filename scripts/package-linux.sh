#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root_dir"

version="$(awk -F '"' '/^version = / { print $2; exit }' Cargo.toml)"
arch="$(uname -m)"
package_name="slate-${version}-linux-${arch}"
stage_dir="$root_dir/dist/$package_name"
archive="$root_dir/dist/${package_name}.tar.gz"

rm -rf "$root_dir/dist"
mkdir -p "$stage_dir"

cargo build --release --locked
make install DESTDIR="$stage_dir" PREFIX=/usr

tar --sort=name \
  --mtime='UTC 2020-01-01' \
  --owner=0 --group=0 --numeric-owner \
  -C "$stage_dir" \
  -czf "$archive" .

sha256sum "$archive" > "${archive}.sha256"
printf 'Created %s\n' "$archive"
