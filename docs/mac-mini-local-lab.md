# Mac mini Local Lab

This runbook records the private, release-binary Local One-click validation for
issue #130 and the operating rules for a personal Apple Silicon Mac mini.

## Validated Baseline

The 2026-07 validation used the official `v0.1.0`
`aarch64-apple-darwin` archive on Apple Silicon, without a Rust toolchain.

| Check | Result |
| --- | --- |
| external archive checksum and bundled `SHA256SUMS` | passed |
| release API health and CLI health | passed |
| MCP initialize, tool discovery, Memory write/search | passed |
| `learning.stem` create/attempt/feedback/list/get | passed |
| Memory, Namespace, and practice data after restart | passed |
| PostgreSQL dump plus Qdrant snapshot isolated restore | passed |
| restored data read through the same release Rust API | passed |

A WeChat message also reached the local release MCP through a temporary,
explicit MiniMax shell helper. That proves a local Adapter chain can reach the
runtime; it is not a native or reboot-persistent MemoryNexus integration. The
MiniMax 3.0.48 external `mavis` CLI incompatibility and a durable WeChat
Adapter remain tracked separately in
[issue #222](https://github.com/blackfaced/MemoryNexus/issues/222).

## Private Network Defaults

The release API and Docker-published dependency ports are localhost-only by
default:

- API: `127.0.0.1:8080`
- PostgreSQL: `127.0.0.1:5432`
- Qdrant HTTP/gRPC: `127.0.0.1:6333` / `127.0.0.1:6334`

Do not expose these ports to a LAN, VPN, or the internet without a separate
deployment review covering authentication, TLS, firewall rules, and backups.

## Install And Start

Download the release archive and matching `.sha256` file, then verify it before
extracting:

```bash
shasum -a 256 -c memorynexus-<tag>-aarch64-apple-darwin.tar.gz.sha256
tar -xzf memorynexus-<tag>-aarch64-apple-darwin.tar.gz
cd memorynexus-<tag>-aarch64-apple-darwin
shasum -a 256 -c SHA256SUMS
```

Create a private runtime env file and replace `JWT_SECRET` with a long random
value. Do not commit this file.

```bash
cp .env.runtime.example .env.runtime
chmod 0600 .env.runtime
```

For a new durable lab, choose one Compose project name and reuse it on every
command and release:

```bash
export MEMORYNEXUS_COMPOSE_PROJECT=memorynexus-local-lab
docker compose -p "$MEMORYNEXUS_COMPOSE_PROJECT" \
  -f docker-compose.runtime.yml --env-file .env.runtime \
  up -d postgres qdrant
```

For an existing lab, first inspect the `com.docker.compose.project` label and
reuse that project name. Changing the project name or bundle directory can
select new named volumes and make existing data appear missing. Do not switch
identity until the old PostgreSQL and Qdrant data have been backed up and
restored together. The repository intentionally does not change existing
Compose project/volume identity in this issue.

Install the binaries and optionally write a client-specific MCP config snippet:

```bash
./install.sh --mcp-config /path/to/private/memorynexus-mcp.json
```

The generated config is forced to mode `0600`. Its default command points to
`~/.local/bin/memorynexus-mcp`; the three installed binaries are:

```text
~/.local/bin/memorynexus
~/.local/bin/memorynexus-cli
~/.local/bin/memorynexus-mcp
```

Load the runtime env and start the API in a long-running terminal:

```bash
set -a
. ./.env.runtime
set +a
~/.local/bin/memorynexus
```

Verify it from another terminal:

```bash
MEMORYNEXUS_API_URL=http://127.0.0.1:8080 \
  ~/.local/bin/memorynexus-cli health
```

This issue does not install launchd. After a reboot, start Docker, the Compose
services with the same project name, the API, and then reload the MCP client.

## Where Data Lives

The Compose volume keys are:

- `memorynexus_runtime_postgres_data`: business records, users and permissions,
  Cognitive Spaces, Namespaces, Memories, Traces, and practice data.
- `memorynexus_runtime_qdrant_data`: vector collections and indexes.

The actual Docker volume names normally include the Compose project prefix.
Use Docker to inspect the selected volumes and container mount destinations:

```bash
docker volume ls
docker inspect <postgres-container> --format '{{json .Mounts}}'
docker inspect <qdrant-container> --format '{{json .Mounts}}'
```

Docker Desktop and Colima run Docker inside a Linux VM on macOS. The reported
volume mountpoint is therefore normally inside that VM, not a supported Finder
folder. Manage and back up the data through PostgreSQL/Qdrant interfaces, not
by copying internal VM directories.

MCP configuration belongs to the specific Adapter (for example its config
directory), and may contain a token. It is separate from Engine data and must
remain private.

## Photo And OCR Boundary

Raw WeChat photos, OCR/ASR output, and media acquisition remain in the Agent or
App Adapter. MemoryNexus receives only normalized text after the user explicitly
accepts or corrects it. Receiving a photo in WeChat does not mean MemoryNexus
stored it. Do not persist the raw photo, full OCR text, credentials, or unrelated
health data as Memory or Trace. This follows
[ADR-021](../decisions/ADR-021-external-media-evidence-references.md).

## Paired Backup And Restore

PostgreSQL and Qdrant together form the recoverable state. A database dump
without the matching vector snapshot is not a complete backup.

1. Stop the API and disable write-capable Adapters.
2. In one maintenance window, create a PostgreSQL custom-format dump with
   `pg_dump -Fc` and a snapshot of `QDRANT_COLLECTION` through Qdrant's
   `POST /collections/{collection}/snapshots` API.
3. Download the snapshot through
   `GET /collections/{collection}/snapshots/{snapshot_name}`.
4. Store both files with the release version, collection name, Compose project
   name, UTC time, and SHA-256 checksums in a private backup directory.
5. Restart writes only after both artifacts and checksums exist.

Keep backup files outside `/tmp`; they contain private user data. Set the backup
directory to `0700` and artifacts to `0600`.

For a restore drill, restore the PostgreSQL dump to a new database with
`pg_restore`, and upload the Qdrant snapshot to a new collection through
`POST /collections/{new_collection}/snapshots/upload?priority=snapshot`. Start
the recorded release API against those isolated names on a different localhost
port. Verify health plus at least one existing Memory, Namespace, and practice
session through the Rust API/MCP. Do not call a restore successful based only
on file existence or row/point counts.

Qdrant's collection snapshot endpoints are documented in the
[official snapshot guide](https://qdrant.tech/documentation/operations/snapshots/).

## Upgrade And Rollback

Before an upgrade:

1. Record the current release version and target.
2. Stop writes and complete the paired backup and restore check above.
3. Keep the verified old release archive and checksum.
4. Start the new release against the existing data using the same Compose
   project identity, then run API, CLI, MCP, Memory, Namespace, and practice
   smoke checks.

The API runs SQLx migrations on startup. Reinstalling only the old binary is
therefore not a safe rollback after a migration. For rollback, stop all writes,
restore the pre-upgrade PostgreSQL dump and Qdrant snapshot as a pair (prefer
fresh database/collection names), reinstall the matching old release, and run
the same application-level smoke before reconnecting Adapters.

## Current Gaps

- No launchd or reboot-persistent service is installed.
- Compose project/volume identity has not been migrated or standardized for
  existing installs.
- MiniMax 3.0.48's removed external CLI is not fixed here.
- The temporary WeChat helper is validation evidence, not a supported product
  feature.
