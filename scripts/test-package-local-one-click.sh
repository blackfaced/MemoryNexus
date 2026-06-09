#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

release_tag="test-v0.0.0"
target="test-target"
binary_dir="$TMP_DIR/bin"
dist_dir="$TMP_DIR/dist"

mkdir -p "$binary_dir" "$dist_dir"
for binary in memorynexus memorynexus-cli memorynexus-mcp; do
  cat > "$binary_dir/$binary" <<'DUMMY_BINARY'
#!/usr/bin/env sh
name=$(basename "$0")
printf '%s %s\n' "$name" "$*" >> "${MEMORYNEXUS_DUMMY_LOG:-/dev/null}"
if [ "$name" = "memorynexus-mcp" ]; then
  input=$(cat)
  printf '%s\n' "$input" >> "${MEMORYNEXUS_DUMMY_STDIN:-/dev/null}"
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"create_space"},{"name":"get_install_status"}]}}'
else
  printf '{"ok":true,"binary":"%s","args":"%s"}\n' "$name" "$*"
fi
DUMMY_BINARY
  chmod 0755 "$binary_dir/$binary"
done

"$ROOT_DIR/scripts/package-local-one-click.sh" \
  --release-tag "$release_tag" \
  --target "$target" \
  --binary-dir "$binary_dir" \
  --dist-dir "$dist_dir"

if "$ROOT_DIR/scripts/package-local-one-click.sh" \
  --release-tag '../bad' \
  --target "$target" \
  --binary-dir "$binary_dir" \
  --dist-dir "$dist_dir" >/dev/null 2>&1; then
  printf 'package script must reject unsafe release tags\n' >&2
  exit 1
fi

archive_base="memorynexus-${release_tag}-${target}"
archive="$dist_dir/${archive_base}.tar.gz"
checksum="$archive.sha256"
test -s "$archive"
test -s "$checksum"

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$dist_dir" && sha256sum -c "${archive_base}.tar.gz.sha256")
else
  (cd "$dist_dir" && shasum -a 256 -c "${archive_base}.tar.gz.sha256")
fi

extract_dir="$TMP_DIR/extract"
mkdir -p "$extract_dir"
tar -C "$extract_dir" -xzf "$archive"
package_dir="$extract_dir/$archive_base"

test -x "$package_dir/bin/memorynexus"
test -x "$package_dir/bin/memorynexus-cli"
test -x "$package_dir/bin/memorynexus-mcp"
test -f "$package_dir/docker-compose.runtime.yml"
test -f "$package_dir/.env.runtime.example"
test -x "$package_dir/install.sh"
test -f "$package_dir/README.local-one-click.md"
test -f "$package_dir/MANIFEST.json"
test -f "$package_dir/SHA256SUMS"

if grep -E '\b(cargo|rustc|rustup)\b' "$package_dir/install.sh" >/dev/null; then
  printf 'install.sh must not call cargo, rustc, or rustup\n' >&2
  exit 1
fi

grep -F '"profile": "local-one-click"' "$package_dir/MANIFEST.json" >/dev/null
grep -F 'bin/memorynexus-mcp' "$package_dir/SHA256SUMS" >/dev/null
grep -F 'docker compose' "$package_dir/README.local-one-click.md" >/dev/null

prefix_dir="$TMP_DIR/prefix with spaces"
mcp_config="$TMP_DIR/mcp config.json"
dummy_log="$TMP_DIR/dummy.log"
dummy_stdin="$TMP_DIR/dummy-stdin.log"

printed_config=$("$package_dir/install.sh" \
  --prefix "$prefix_dir" \
  --api-url 'http://localhost:8080/quoted"api' \
  --token 'token"with\backslash' \
  --print-mcp-config)
printf '%s\n' "$printed_config" | grep -F "$prefix_dir/bin/memorynexus-mcp" >/dev/null

"$package_dir/install.sh" \
  --prefix "$prefix_dir" \
  --api-url 'http://localhost:8080' \
  --token 'token"with\backslash' \
  --mcp-config "$mcp_config" >/dev/null
python3 -m json.tool "$mcp_config" >/dev/null
grep -F "$prefix_dir/bin/memorynexus-mcp" "$mcp_config" >/dev/null

MEMORYNEXUS_DUMMY_LOG="$dummy_log" MEMORYNEXUS_DUMMY_STDIN="$dummy_stdin" \
  "$package_dir/install.sh" \
  --prefix "$prefix_dir" \
  --api-url 'http://localhost:8080' \
  --token 'placeholder-token' \
  --smoke >/dev/null
grep -F 'memorynexus-cli health' "$dummy_log" >/dev/null
grep -F 'memorynexus-mcp ' "$dummy_log" >/dev/null
grep -F '"tools/list"' "$dummy_stdin" >/dev/null

printf 'local one-click bundle packaging smoke passed\n'
