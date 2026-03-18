#!/usr/bin/env bash
set -euo pipefail

REPO="KuangjuX/ncu-cli"
BINARY="ncu-cli"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# ── Helpers ──────────────────────────────────────────────────────────

info()  { printf '\033[1;34m[info]\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m[warn]\033[0m  %s\n' "$*"; }
error() { printf '\033[1;31m[error]\033[0m %s\n' "$*" >&2; exit 1; }

need() {
    command -v "$1" >/dev/null 2>&1 || error "'$1' is required but not found. Please install it first."
}

# ── Detect platform ──────────────────────────────────────────────────

detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux*)  OS="linux"  ;;
        Darwin*) OS="darwin" ;;
        *)       error "Unsupported OS: $os (only Linux and macOS are supported)" ;;
    esac

    case "$arch" in
        x86_64|amd64)   ARCH="x86_64"  ;;
        aarch64|arm64)  ARCH="aarch64" ;;
        *)              error "Unsupported architecture: $arch" ;;
    esac

    info "Detected platform: ${OS}-${ARCH}"
}

# ── Resolve version ─────────────────────────────────────────────────

resolve_version() {
    if [ -n "${VERSION:-}" ]; then
        TAG="v${VERSION#v}"
        info "Using specified version: $TAG"
        return
    fi

    TAG=""
    info "No version specified — will build from the latest main branch."
}

# ── Build from source ───────────────────────────────────────────────

build_from_source() {
    info "Building from source..."

    local tmpdir
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    info "Cloning https://github.com/${REPO}.git ..."
    if [ -n "$TAG" ]; then
        git clone --depth 1 --branch "$TAG" "https://github.com/${REPO}.git" "$tmpdir/src"
    else
        git clone --depth 1 "https://github.com/${REPO}.git" "$tmpdir/src"
    fi

    info "Compiling (release mode)..."
    cargo build --release --manifest-path "$tmpdir/src/Cargo.toml"

    local bin_path="$tmpdir/src/target/release/$BINARY"
    [ -f "$bin_path" ] || error "Build succeeded but binary not found at $bin_path"

    install_binary "$bin_path"
}

# ── Install binary to destination ────────────────────────────────────

install_binary() {
    local src="$1"

    info "Installing $BINARY to $INSTALL_DIR ..."

    mkdir -p "$INSTALL_DIR" 2>/dev/null || true

    if [ -w "$INSTALL_DIR" ]; then
        cp "$src" "$INSTALL_DIR/$BINARY"
        chmod +x "$INSTALL_DIR/$BINARY"
    else
        warn "$INSTALL_DIR is not writable — using sudo"
        sudo mkdir -p "$INSTALL_DIR"
        sudo cp "$src" "$INSTALL_DIR/$BINARY"
        sudo chmod +x "$INSTALL_DIR/$BINARY"
    fi

    info "Installed successfully: $INSTALL_DIR/$BINARY"
}

# ── Verify installation ─────────────────────────────────────────────

verify() {
    if command -v "$BINARY" >/dev/null 2>&1; then
        info "$("$BINARY" --version 2>/dev/null || echo "$BINARY installed")"
        info "Run '$BINARY --help' to get started."
    else
        warn "$BINARY was installed to $INSTALL_DIR but is not on your PATH."
        warn "Add the following to your shell profile:"
        warn "  export PATH=\"$INSTALL_DIR:\$PATH\""
    fi
}

# ── Main ─────────────────────────────────────────────────────────────

main() {
    info "ncu-cli installer (build from source)"
    echo

    need "git"
    need "cargo"

    detect_platform
    resolve_version
    build_from_source
    verify

    echo
    info "Done!"
}

main "$@"
