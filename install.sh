#!/usr/bin/env bash
# Installs the latest oxr release to /usr/local/bin (override with OXR_INSTALL_DIR).
# Usage: curl -fsSL https://raw.githubusercontent.com/oxhive/oxr/main/install.sh | sh
set -euo pipefail

repo="oxhive/oxr"
bin_dir="${OXR_INSTALL_DIR:-/usr/local/bin}"
version="${1:-latest}"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)   target=x86_64-unknown-linux-gnu ;;
  Linux-aarch64)  target=aarch64-unknown-linux-gnu ;;
  Darwin-arm64)   target=aarch64-apple-darwin ;;
  *)
    echo "oxr has no prebuilt binary for $(uname -s)-$(uname -m). Supported: Linux x86_64/aarch64, macOS arm64." >&2
    exit 1
    ;;
esac

if [ "$version" = "latest" ]; then
  url="https://github.com/${repo}/releases/latest/download/oxr-${target}.tar.gz"
else
  url="https://github.com/${repo}/releases/download/${version}/oxr-${target}.tar.gz"
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

echo "Downloading $url"
curl -fsSL "$url" -o "$tmp_dir/oxr.tar.gz"
tar -xzf "$tmp_dir/oxr.tar.gz" -C "$tmp_dir"
chmod +x "$tmp_dir/oxr"

mkdir -p "$bin_dir" 2>/dev/null || sudo mkdir -p "$bin_dir"
if [ -w "$bin_dir" ]; then
  mv "$tmp_dir/oxr" "$bin_dir/oxr"
else
  sudo mv "$tmp_dir/oxr" "$bin_dir/oxr"
fi

echo "Installed $("$bin_dir/oxr" --version) to ${bin_dir}/oxr"
