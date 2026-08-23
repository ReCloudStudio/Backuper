#!/usr/bin/env bash
set -euo pipefail

PREFIX="/usr/local"
CONFIG_DIR="/etc/backuper"
DATA_DIR="/var/lib/backuper"
USER_NAME="backuper"

cd "$(dirname "$0")/.."

if command -v bun >/dev/null 2>&1; then
    echo "Building WebUI..."
    (cd webui && bun install && bun run build)
else
    echo "bun not found; WebUI will not be embedded"
fi

echo "Building release binaries..."
cargo build --release --bin backuperd --bin backuperctl

echo "Installing binaries..."
install -Dm755 target/release/backuperd "${PREFIX}/bin/backuperd"
install -Dm755 target/release/backuperctl "${PREFIX}/bin/backuperctl"

echo "Installing systemd service..."
install -Dm644 systemd/backuper.service /etc/systemd/system/backuper.service

echo "Creating user and directories..."
if ! id -u "${USER_NAME}" >/dev/null 2>&1; then
    useradd --system --no-create-home --home-dir "${DATA_DIR}" "${USER_NAME}"
fi

install -dDm755 "${CONFIG_DIR}"
install -dDm750 "${DATA_DIR}" -o "${USER_NAME}" -g "${USER_NAME}"

if [[ ! -f "${CONFIG_DIR}/backuper.toml" ]]; then
    install -Dm644 examples/backuper.toml "${CONFIG_DIR}/backuper.toml"
fi

echo "Reloading systemd..."
systemctl daemon-reload

echo "Done."
echo "Edit ${CONFIG_DIR}/backuper.toml, then run:"
echo "  systemctl enable --now backuper"
