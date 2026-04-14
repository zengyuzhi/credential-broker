#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <tag>" >&2
  exit 1
fi

TAG="$1"
REPO="${GH_REPO:-zengyuzhi/credential-broker}"
SECRET_KEY="${MINISIGN_SECRET_KEY:-$HOME/.minisign/vault.key}"
PUBKEY_PATH="${MINISIGN_PUBLIC_KEY_PATH:-crates/vault-cli/release-pubkey.minisign}"

require() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: missing required command: $1" >&2
    exit 1
  }
}

require gh
require minisign

if [[ ! -f "$SECRET_KEY" ]]; then
  echo "error: secret key not found at $SECRET_KEY" >&2
  exit 1
fi

if [[ ! -f "$PUBKEY_PATH" ]]; then
  echo "error: public key not found at $PUBKEY_PATH" >&2
  exit 1
fi

KEY_ID="$(tail -n 1 "$PUBKEY_PATH" | tr -d '\r\n')"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

IS_DRAFT="$(gh release view "$TAG" --repo "$REPO" --json isDraft --jq '.isDraft')"
if [[ "$IS_DRAFT" != "true" ]]; then
  echo "error: release $TAG is not a draft; refusing to sign/publish" >&2
  exit 1
fi

gh release download "$TAG" --repo "$REPO" --pattern SHA256SUMS --dir "$TMPDIR"
minisign -Sm "$TMPDIR/SHA256SUMS" -s "$SECRET_KEY"
gh release upload "$TAG" --repo "$REPO" "$TMPDIR/SHA256SUMS.minisig" --clobber
minisign -Vm "$TMPDIR/SHA256SUMS" -p "$PUBKEY_PATH" -x "$TMPDIR/SHA256SUMS.minisig"

gh release view "$TAG" --repo "$REPO" --json body --jq '.body // ""' > "$TMPDIR/release-body.md"
if ! grep -Fq "signed by minisign key \`$KEY_ID\`" "$TMPDIR/release-body.md"; then
  printf '\n\nsigned by minisign key `%s`\n' "$KEY_ID" >> "$TMPDIR/release-body.md"
fi

gh release edit "$TAG" \
  --repo "$REPO" \
  --draft=false \
  --notes-file "$TMPDIR/release-body.md"

echo "Signed and published $TAG with minisign key $KEY_ID"
