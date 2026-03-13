#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
TAURI_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
WORKSPACE_DIR="$(cd -- "${TAURI_DIR}/../../.." && pwd)"

TARGET_TRIPLE="${TAURI_ENV_TARGET_TRIPLE:-}"
if [[ -z "${TARGET_TRIPLE}" ]]; then
  TARGET_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
fi

if [[ -z "${TARGET_TRIPLE}" ]]; then
  echo "failed to resolve target triple"
  exit 1
fi

EXT=""
if [[ "${TARGET_TRIPLE}" == *windows* ]]; then
  EXT=".exe"
fi

echo "[build] web dist"
(cd "${WORKSPACE_DIR}/apps/desktop/web" && npm run build)

echo "[build] backend (${TARGET_TRIPLE})"
(cd "${WORKSPACE_DIR}" && cargo build -p desktop-ai-backend --release --target "${TARGET_TRIPLE}")

SOURCE_BIN="${WORKSPACE_DIR}/target/${TARGET_TRIPLE}/release/desktop-ai-backend${EXT}"
if [[ ! -f "${SOURCE_BIN}" ]]; then
  SOURCE_BIN="${WORKSPACE_DIR}/target/release/desktop-ai-backend${EXT}"
fi

if [[ ! -f "${SOURCE_BIN}" ]]; then
  echo "backend binary not found: ${SOURCE_BIN}"
  exit 1
fi

BIN_DIR="${TAURI_DIR}/binaries"
mkdir -p "${BIN_DIR}"
DEST_BIN="${BIN_DIR}/desktop-ai-backend-${TARGET_TRIPLE}${EXT}"
cp "${SOURCE_BIN}" "${DEST_BIN}"

if [[ "${EXT}" != ".exe" ]]; then
  chmod +x "${DEST_BIN}"
fi

echo "[ok] sidecar prepared: ${DEST_BIN}"
