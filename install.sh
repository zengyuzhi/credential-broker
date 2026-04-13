#!/usr/bin/env bash
set -euo pipefail

# credential-broker installer
# Usage: curl -fsSL https://raw.githubusercontent.com/zengyuzhi/credential-broker/main/install.sh | bash

REPO="zengyuzhi/credential-broker"
INSTALL_DIR="${HOME}/.local/bin"
BINARY_NAME="vault"

# Colors (if terminal supports them)
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    NC='\033[0m'
else
    RED='' GREEN='' YELLOW='' NC=''
fi

info()  { echo -e "${GREEN}$*${NC}"; }
warn()  { echo -e "${YELLOW}$*${NC}"; }
error() { echo -e "${RED}error: $*${NC}" >&2; exit 1; }

# --- Platform checks ---

OS="$(uname -s)"
if [ "$OS" != "Darwin" ]; then
    error "credential-broker requires macOS (Keychain integration). See https://github.com/${REPO} for details."
fi

ARCH="$(uname -m)"
case "$ARCH" in
    arm64|aarch64) TARGET="aarch64-apple-darwin" ;;
    x86_64)        TARGET="x86_64-apple-darwin" ;;
    *)             error "Unsupported architecture: ${ARCH}" ;;
esac

# --- Fetch latest release ---

info "Detecting latest release..."
LATEST_URL="https://api.github.com/repos/${REPO}/releases/latest"
RELEASE_JSON="$(curl -fsSL "$LATEST_URL" 2>/dev/null)" || error "Failed to fetch release info. Check your internet connection."

VERSION="$(echo "$RELEASE_JSON" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\(.*\)".*/\1/')"
if [ -z "$VERSION" ]; then
    error "Could not determine latest version. Visit https://github.com/${REPO}/releases"
fi

info "Latest version: ${VERSION}"

# --- Download binary ---

ASSET_NAME="vault-${TARGET}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET_NAME}"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

info "Downloading ${ASSET_NAME}..."
curl -fsSL "$DOWNLOAD_URL" -o "${TMPDIR}/${ASSET_NAME}" || error "Download failed. Asset may not exist for ${TARGET} in ${VERSION}."

# --- Install ---

mkdir -p "$INSTALL_DIR"

info "Installing to ${INSTALL_DIR}/${BINARY_NAME}..."
tar xzf "${TMPDIR}/${ASSET_NAME}" -C "$INSTALL_DIR"
chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

# --- Verify ---

INSTALLED_PATH="${INSTALL_DIR}/${BINARY_NAME}"
if [ ! -x "$INSTALLED_PATH" ]; then
    error "Installation failed — binary not found at ${INSTALLED_PATH}"
fi

info "vault ${VERSION} installed to ${INSTALLED_PATH}"

# --- PATH check ---

case ":$PATH:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
        warn ""
        warn "Add ${INSTALL_DIR} to your PATH:"
        warn ""
        warn "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc"
        warn "  source ~/.zshrc"
        warn ""
        ;;
esac

info ""
info "Run 'vault --help' to get started."
