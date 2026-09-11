#!/usr/bin/env bash
# Castle installer — https://github.com/VJ-2303/starNotes
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/VJ-2303/starNotes/main/install.sh | bash
#   bash install.sh                  # install latest
#   CASTLE_VERSION=v0.4.0 bash install.sh   # pin a version
#   CASTLE_INSTALL_DIR=/usr/local/bin bash install.sh  # custom install dir (needs sudo for system paths)

set -euo pipefail

# ── Configuration ────────────────────────────────────────────────────────────
REPO="VJ-2303/starNotes"
BINARY_NAME="castle"
INSTALL_DIR="${CASTLE_INSTALL_DIR:-$HOME/.local/bin}"

# ── Helpers ───────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BOLD='\033[1m'; RESET='\033[0m'

info()    { echo -e "${BOLD}[castle]${RESET} $*"; }
success() { echo -e "${GREEN}[castle]${RESET} $*"; }
warn()    { echo -e "${YELLOW}[castle]${RESET} $*" >&2; }
die()     { echo -e "${RED}[castle] error:${RESET} $*" >&2; exit 1; }

need() {
  command -v "$1" &>/dev/null || die "'$1' is required but not installed. Install it and retry."
}

# ── System checks ─────────────────────────────────────────────────────────────
[ "$(uname -s)" = "Linux" ] || die "Castle only supports Linux."

need curl
need tar

ARCH="$(uname -m)"
case "$ARCH" in
  x86_64) ;;
  *) die "Unsupported architecture '$ARCH'. Only x86_64 is supported." ;;
esac

# ── Resolve version ───────────────────────────────────────────────────────────
if [ -n "${CASTLE_VERSION:-}" ]; then
  TAG="$CASTLE_VERSION"
  info "Installing Castle $TAG (pinned)..."
else
  info "Fetching latest Castle release..."
  TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' \
    | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
  [ -n "$TAG" ] || die "Could not determine latest release. Check your internet connection."
  info "Latest release: $TAG"
fi

VERSION="${TAG#v}"

# ── Download ──────────────────────────────────────────────────────────────────
ARCHIVE="Castle-$VERSION-linux-$ARCH.tar.gz"
URL="https://github.com/$REPO/releases/download/$TAG/$ARCHIVE"
CHECKSUMS_URL="https://github.com/$REPO/releases/download/$TAG/SHA256SUMS.txt"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

info "Downloading $ARCHIVE..."
curl -fSL --progress-bar "$URL" -o "$TMP_DIR/$ARCHIVE"

# ── Verify checksum ───────────────────────────────────────────────────────────
info "Verifying checksum..."
CHECKSUMS_FILE="$TMP_DIR/SHA256SUMS.txt"
if curl -fsSL "$CHECKSUMS_URL" -o "$CHECKSUMS_FILE" 2>/dev/null; then
  # Filter to just the line for our archive and verify
  EXPECTED=$(grep " $ARCHIVE$" "$CHECKSUMS_FILE" | awk '{print $1}')
  if [ -n "$EXPECTED" ]; then
    ACTUAL=$(sha256sum "$TMP_DIR/$ARCHIVE" | awk '{print $1}')
    if [ "$EXPECTED" != "$ACTUAL" ]; then
      die "Checksum mismatch!\n  expected: $EXPECTED\n  actual:   $ACTUAL\nThe download may be corrupted."
    fi
    success "Checksum OK."
  else
    warn "No checksum entry found for $ARCHIVE — skipping verification."
  fi
else
  warn "Could not fetch SHA256SUMS.txt — skipping verification."
fi

# ── Extract and install ───────────────────────────────────────────────────────
info "Extracting..."
tar -xzf "$TMP_DIR/$ARCHIVE" -C "$TMP_DIR"

EXTRACTED="$TMP_DIR/$BINARY_NAME"
[ -f "$EXTRACTED" ] || die "Expected binary '$BINARY_NAME' not found in archive."

mkdir -p "$INSTALL_DIR"
install -m 755 "$EXTRACTED" "$INSTALL_DIR/$BINARY_NAME"

# ── PATH hint ─────────────────────────────────────────────────────────────────
INSTALLED_AT="$INSTALL_DIR/$BINARY_NAME"
success "Castle $TAG installed → $INSTALLED_AT"

if ! echo ":$PATH:" | grep -q ":$INSTALL_DIR:"; then
  echo ""
  warn "$INSTALL_DIR is not in your PATH."
  warn "Add the following line to your shell config (~/.bashrc, ~/.zshrc, etc.):"
  echo ""
  echo "    export PATH=\"\$PATH:$INSTALL_DIR\""
  echo ""
fi

echo ""
info "Run 'castle' to start."
