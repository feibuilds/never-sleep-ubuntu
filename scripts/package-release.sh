#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "usage: $0 <version>" >&2
    exit 2
fi

project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
version=${1#v}
archive_name="never-sleep-ubuntu-${version}-linux-x86_64"
stage_dir=$(mktemp -d)
trap 'rm -rf -- "$stage_dir"' EXIT

cd "$project_dir"
test -x target/release/never-sleep
install -d "dist" "$stage_dir/$archive_name/bin" "$stage_dir/$archive_name/share/applications"
install -m 0755 target/release/never-sleep "$stage_dir/$archive_name/bin/"
install -m 0755 scripts/tray.py "$stage_dir/$archive_name/bin/never-sleep-tray"
install -m 0644 packaging/never-sleep.desktop "$stage_dir/$archive_name/share/applications/"
install -m 0644 README.md README.zh-CN.md LICENSE "$stage_dir/$archive_name/"
tar -C "$stage_dir" -czf "dist/$archive_name.tar.gz" "$archive_name"
sha256sum "dist/$archive_name.tar.gz" >"dist/$archive_name.tar.gz.sha256"
