#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: package-local-one-click.sh --release-tag <tag> --target <target> --binary-dir <dir> [--dist-dir <dir>]

Builds the MemoryNexus Local One-click release archive layout from prebuilt
binaries. The output archive is offline-friendly for MemoryNexus binaries and
runtime files, but Docker may still need to pull PostgreSQL and Qdrant images.
USAGE
}

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
release_tag=""
target=""
binary_dir=""
dist_dir="$ROOT_DIR/dist"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --release-tag)
      release_tag=${2:-}
      shift 2
      ;;
    --target)
      target=${2:-}
      shift 2
      ;;
    --binary-dir)
      binary_dir=${2:-}
      shift 2
      ;;
    --dist-dir)
      dist_dir=${2:-}
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'unknown argument: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [ -z "$release_tag" ] || [ -z "$target" ] || [ -z "$binary_dir" ]; then
  usage >&2
  exit 2
fi

if [[ ! "$release_tag" =~ ^[A-Za-z0-9._-]+$ ]]; then
  printf 'release tag must contain only letters, numbers, dot, underscore, or dash: %s\n' "$release_tag" >&2
  exit 2
fi

if [[ ! "$target" =~ ^[A-Za-z0-9._-]+$ ]]; then
  printf 'target must contain only letters, numbers, dot, underscore, or dash: %s\n' "$target" >&2
  exit 2
fi

for binary in memorynexus memorynexus-cli memorynexus-mcp; do
  if [ ! -f "$binary_dir/$binary" ]; then
    printf 'missing required binary: %s\n' "$binary_dir/$binary" >&2
    exit 1
  fi
done

if [ ! -f "$ROOT_DIR/docker-compose.runtime.yml" ]; then
  printf 'missing docker-compose.runtime.yml\n' >&2
  exit 1
fi

if [ ! -f "$ROOT_DIR/.env.runtime.example" ]; then
  printf 'missing .env.runtime.example\n' >&2
  exit 1
fi

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$@"
  else
    shasum -a 256 "$@"
  fi
}

archive_base="memorynexus-${release_tag}-${target}"
package_dir="$dist_dir/$archive_base"
rm -rf "$package_dir" "$dist_dir/${archive_base}.tar.gz" "$dist_dir/${archive_base}.tar.gz.sha256"
mkdir -p "$package_dir/bin"

cp "$binary_dir/memorynexus" "$package_dir/bin/"
cp "$binary_dir/memorynexus-cli" "$package_dir/bin/"
cp "$binary_dir/memorynexus-mcp" "$package_dir/bin/"
cp "$ROOT_DIR/docker-compose.runtime.yml" "$package_dir/"
cp "$ROOT_DIR/.env.runtime.example" "$package_dir/"
chmod 0755 \
  "$package_dir/bin/memorynexus" \
  "$package_dir/bin/memorynexus-cli" \
  "$package_dir/bin/memorynexus-mcp"

cat > "$package_dir/install.sh" <<'INSTALL_SH'
#!/usr/bin/env bash
set -euo pipefail

PACKAGE_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
PREFIX="${HOME}/.local"
API_URL="${MEMORYNEXUS_API_URL:-http://localhost:8080}"
TOKEN="${MEMORYNEXUS_TOKEN:-}"
MCP_CONFIG_PATH=""
PRINT_MCP_CONFIG=false
START_SERVICES=false
RUN_SMOKE=false
ENV_FILE="$PACKAGE_DIR/.env.runtime"

usage() {
  cat <<'USAGE'
Usage: ./install.sh [options]

Options:
  --prefix <dir>          Install binaries under <dir>/bin (default: ~/.local)
  --api-url <url>         MemoryNexus API URL (default: http://localhost:8080)
  --token <token>         Token for MCP config and smoke checks
  --mcp-config <path>     Write a MemoryNexus MCP config JSON snippet to path
  --print-mcp-config      Print the MCP config JSON snippet to stdout
  --start-services        Start PostgreSQL and Qdrant with bundled Docker Compose
  --smoke                 Run API health and MCP tools/list smoke checks
  --env-file <path>       Runtime env file for Docker Compose (default: bundle .env.runtime)
  -h, --help              Show this help

The script installs prebuilt MemoryNexus binaries and can start/check local
PostgreSQL and Qdrant services. It expects Docker for Local One-click services.
USAGE
}

json_escape() {
  local value=${1-}
  value=${value//\\/\\\\}
  value=${value//\"/\\\"}
  value=${value//$'\n'/\\n}
  value=${value//$'\r'/\\r}
  value=${value//$'\t'/\\t}
  printf '%s' "$value"
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --prefix)
      PREFIX=${2:-}
      shift 2
      ;;
    --api-url)
      API_URL=${2:-}
      shift 2
      ;;
    --token)
      TOKEN=${2:-}
      shift 2
      ;;
    --mcp-config)
      MCP_CONFIG_PATH=${2:-}
      shift 2
      ;;
    --print-mcp-config)
      PRINT_MCP_CONFIG=true
      shift
      ;;
    --start-services)
      START_SERVICES=true
      shift
      ;;
    --smoke)
      RUN_SMOKE=true
      shift
      ;;
    --env-file)
      ENV_FILE=${2:-}
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'unknown argument: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

need_docker=false
if [ "$START_SERVICES" = true ]; then
  need_docker=true
fi

if [ "$need_docker" = true ]; then
  if ! command -v docker >/dev/null 2>&1; then
    printf 'Docker is required for Local One-click PostgreSQL and Qdrant services.\n' >&2
    exit 1
  fi
  docker version >/dev/null
  docker compose version >/dev/null
fi

install_dir="$PREFIX/bin"
mkdir -p "$install_dir"
cp "$PACKAGE_DIR/bin/memorynexus" "$install_dir/"
cp "$PACKAGE_DIR/bin/memorynexus-cli" "$install_dir/"
cp "$PACKAGE_DIR/bin/memorynexus-mcp" "$install_dir/"
chmod 0755 "$install_dir/memorynexus" "$install_dir/memorynexus-cli" "$install_dir/memorynexus-mcp"

if [ ! -f "$ENV_FILE" ]; then
  cp "$PACKAGE_DIR/.env.runtime.example" "$ENV_FILE"
fi

if [ "$START_SERVICES" = true ]; then
  docker compose \
    -f "$PACKAGE_DIR/docker-compose.runtime.yml" \
    --env-file "$ENV_FILE" \
    up -d postgres qdrant
fi

escaped_command=$(json_escape "$install_dir/memorynexus-mcp")
escaped_api_url=$(json_escape "$API_URL")
escaped_token=$(json_escape "${TOKEN:-<jwt-token>}")

mcp_json=$(cat <<JSON
{
  "mcpServers": {
    "memorynexus": {
      "command": "$escaped_command",
      "env": {
        "MEMORYNEXUS_API_URL": "$escaped_api_url",
        "MEMORYNEXUS_TOKEN": "$escaped_token"
      }
    }
  }
}
JSON
)

if [ -n "$MCP_CONFIG_PATH" ]; then
  mkdir -p "$(dirname "$MCP_CONFIG_PATH")"
  printf '%s\n' "$mcp_json" > "$MCP_CONFIG_PATH"
  printf 'Wrote MCP config snippet to %s\n' "$MCP_CONFIG_PATH"
fi

if [ "$PRINT_MCP_CONFIG" = true ]; then
  printf '%s\n' "$mcp_json"
fi

if [ "$RUN_SMOKE" = true ]; then
  MEMORYNEXUS_API_URL="$API_URL" "$install_dir/memorynexus-cli" health
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' \
    | MEMORYNEXUS_API_URL="$API_URL" MEMORYNEXUS_TOKEN="${TOKEN:-placeholder-token}" "$install_dir/memorynexus-mcp"
fi

cat <<EOF
MemoryNexus Local One-click files are installed.

Binaries:
  $install_dir/memorynexus
  $install_dir/memorynexus-cli
  $install_dir/memorynexus-mcp

Runtime env file:
  $ENV_FILE

Next steps:
  1. If not started yet: ./install.sh --start-services
  2. Load env values from $ENV_FILE and run: $install_dir/memorynexus
  3. Run health: MEMORYNEXUS_API_URL=$API_URL $install_dir/memorynexus-cli health
  4. Add the printed or written MCP config to your agent client and reload it.
EOF
INSTALL_SH
chmod 0755 "$package_dir/install.sh"

cat > "$package_dir/README.local-one-click.md" <<'README_MD'
# MemoryNexus Local One-click Bundle

This archive is the Local One-click Profile for MemoryNexus. It is intended for
local-first use without compiling Rust. The bundle includes prebuilt binaries,
the runtime Docker Compose file for PostgreSQL and Qdrant, an env example, an
install script, checksums, and manifest metadata.

## What Is Included

- `bin/memorynexus` — Rust API binary
- `bin/memorynexus-cli` — CLI client
- `bin/memorynexus-mcp` — MCP stdio server for agent clients
- `docker-compose.runtime.yml` — PostgreSQL and Qdrant runtime services
- `.env.runtime.example` — runtime env values for Docker services and the API
- `install.sh` — local install helper; it does not compile MemoryNexus
- `SHA256SUMS` and `MANIFEST.json` — bundle verification metadata

The bundle does not include Docker image tarballs, database snapshots, a GUI
installer, or a second backend.

## Verify The Archive

Before extracting, verify the downloaded archive checksum:

```bash
sha256sum -c memorynexus-<tag>-<target>.tar.gz.sha256
```

On macOS:

```bash
shasum -a 256 -c memorynexus-<tag>-<target>.tar.gz.sha256
```

After extracting, verify files inside the bundle:

```bash
cd memorynexus-<tag>-<target>
sha256sum -c SHA256SUMS
```

On macOS:

```bash
shasum -a 256 -c SHA256SUMS
```

## Install Binaries

Install binaries under `~/.local/bin`:

```bash
./install.sh --print-mcp-config
```

Use a different prefix if needed:

```bash
./install.sh --prefix /opt/memorynexus --print-mcp-config
```

The script can write an MCP config snippet for an agent client:

```bash
./install.sh \
  --api-url http://localhost:8080 \
  --token "$MEMORYNEXUS_TOKEN" \
  --mcp-config ./memorynexus-mcp-config.json
```

Do not paste real tokens into chat or commit MCP config files containing tokens.

## Start Local Runtime Services

Local One-click uses Docker only for PostgreSQL and Qdrant. It does not build or
run the MemoryNexus API in Docker.

```bash
./install.sh --start-services
```

Equivalent manual command:

```bash
docker compose \
  -f docker-compose.runtime.yml \
  --env-file .env.runtime.example \
  up -d postgres qdrant
```

If Docker image pulling fails, configure Docker Desktop or the Docker daemon
proxy. This first slice does not solve fully air-gapped Docker image delivery.

## Start The API Binary

Load runtime values, then start the API:

```bash
set -a
. ./.env.runtime.example
set +a

./bin/memorynexus
```

If you used `install.sh`, you can also run the installed binary:

```bash
~/.local/bin/memorynexus
```

The API listens on `http://localhost:8080` by default.

## Smoke Checks

API health:

```bash
MEMORYNEXUS_API_URL=http://localhost:8080 ./bin/memorynexus-cli health
```

MCP `tools/list` smoke:

```bash
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' \
  | MEMORYNEXUS_API_URL=http://localhost:8080 \
    MEMORYNEXUS_TOKEN="${MEMORYNEXUS_TOKEN:-placeholder-token}" \
    ./bin/memorynexus-mcp
```

The response should include MemoryNexus tools such as `create_space`,
`add_memory`, `search_memories`, `get_install_status`, and `upgrade_install`.

## Profile Guidance

- Trial Profile is lighter: use only `memorynexus-mcp` with an existing hosted
  or demo API. It avoids local Docker, PostgreSQL, and Qdrant.
- Local One-click Profile is local-first: use this archive plus Docker-managed
  PostgreSQL and Qdrant on the same machine.
- Production Profile is better for serious hosted use: run stable API binaries
  against managed or self-hosted PostgreSQL/Qdrant with backups, TLS, and
  monitoring.
README_MD

generated_at=$(date -u '+%Y-%m-%dT%H:%M:%SZ')
cat > "$package_dir/MANIFEST.json" <<EOF
{
  "name": "MemoryNexus Local One-click Bundle",
  "profile": "local-one-click",
  "release_tag": "$release_tag",
  "target": "$target",
  "generated_at": "$generated_at",
  "contents": [
    "bin/memorynexus",
    "bin/memorynexus-cli",
    "bin/memorynexus-mcp",
    "docker-compose.runtime.yml",
    ".env.runtime.example",
    "install.sh",
    "README.local-one-click.md",
    "SHA256SUMS",
    "MANIFEST.json"
  ],
  "runtime_services": ["postgres", "qdrant"],
  "non_goals": [
    "fully air-gapped Docker image distribution",
    "database snapshots",
    "GUI installer",
    "Production Profile replacement",
    "second backend"
  ]
}
EOF

(
  cd "$package_dir"
  sha256_file \
    bin/memorynexus \
    bin/memorynexus-cli \
    bin/memorynexus-mcp \
    docker-compose.runtime.yml \
    .env.runtime.example \
    install.sh \
    README.local-one-click.md \
    MANIFEST.json > SHA256SUMS
)

tar -C "$dist_dir" -czf "$dist_dir/${archive_base}.tar.gz" "$archive_base"
(
  cd "$dist_dir"
  sha256_file "${archive_base}.tar.gz" > "${archive_base}.tar.gz.sha256"
)

printf 'Wrote %s\n' "$dist_dir/${archive_base}.tar.gz"
printf 'Wrote %s\n' "$dist_dir/${archive_base}.tar.gz.sha256"
