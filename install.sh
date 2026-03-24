#!/bin/sh
# rez-next installer for Linux and macOS
# Usage: curl -fsSL https://raw.githubusercontent.com/loonghao/rez-next/main/install.sh | sh
#
# Environment variables:
#   REZ_NEXT_VERSION  - Specific version to install (e.g., "0.1.0"). Default: latest
#   REZ_NEXT_INSTALL  - Installation directory. Default: $HOME/.rez-next/bin
#   REZ_NEXT_MUSL     - Set to "1" on Linux to prefer musl build. Default: auto-detect
set -eu

REPO="loonghao/rez-next"
BINARY_NAME="rez-next"

# --- Logging helpers ---
info() { printf '\033[0;34minfo:\033[0m %s\n' "$*"; }
success() { printf '\033[0;32msuccess:\033[0m %s\n' "$*"; }
warn() { printf '\033[0;33mwarn:\033[0m %s\n' "$*"; }
err() { printf '\033[0;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

# --- Detect downloader ---
detect_downloader() {
    if command -v curl >/dev/null 2>&1; then
        DOWNLOADER="curl"
    elif command -v wget >/dev/null 2>&1; then
        DOWNLOADER="wget"
    else
        err "Neither curl nor wget found. Please install one of them."
    fi
}

download() {
    local url="$1" dest="$2"
    if [ "$DOWNLOADER" = "curl" ]; then
        curl -fsSL "$url" -o "$dest"
    else
        wget -qO "$dest" "$url"
    fi
}

download_text() {
    local url="$1"
    if [ "$DOWNLOADER" = "curl" ]; then
        curl -fsSL "$url"
    else
        wget -qO- "$url"
    fi
}

# --- Detect OS ---
detect_os() {
    local uname_s
    uname_s=$(uname -s)
    case "$uname_s" in
        Linux*)  OS="linux" ;;
        Darwin*) OS="darwin" ;;
        MINGW*|MSYS*|CYGWIN*) OS="windows" ;;
        *) err "Unsupported operating system: $uname_s" ;;
    esac
}

# --- Detect architecture ---
detect_arch() {
    local uname_m
    uname_m=$(uname -m)
    case "$uname_m" in
        x86_64|amd64) ARCH="x86_64" ;;
        aarch64|arm64) ARCH="aarch64" ;;
        *) err "Unsupported architecture: $uname_m" ;;
    esac
}

# --- Detect target triple ---
detect_target() {
    detect_os
    detect_arch

    case "$OS" in
        linux)
            # Determine libc type
            local use_musl="${REZ_NEXT_MUSL:-0}"
            if [ "$use_musl" = "1" ]; then
                LIBC="musl"
            elif command -v ldd >/dev/null 2>&1 && ldd --version 2>&1 | grep -qi musl; then
                LIBC="musl"
            elif [ -f /etc/os-release ]; then
                # Detect musl-based distros like Alpine, Void Linux
                . /etc/os-release
                case "$ID" in
                    alpine|void) LIBC="musl" ;;
                    *) LIBC="gnu" ;;
                esac
            else
                LIBC="gnu"
            fi
            TARGET="${ARCH}-unknown-linux-${LIBC}"
            ;;
        darwin)
            TARGET="${ARCH}-apple-darwin"
            ;;
        windows)
            TARGET="${ARCH}-pc-windows-msvc"
            ;;
    esac

    info "Detected platform: $OS $ARCH ($TARGET)"
}

# --- Get latest version ---
get_latest_version() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"
    local response
    response=$(download_text "$url" 2>/dev/null) || err "Failed to fetch latest version from GitHub API"

    # Parse tag_name from JSON response (simple grep approach, no jq dependency)
    VERSION=$(echo "$response" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/' | sed 's/^v//')

    if [ -z "$VERSION" ]; then
        err "Failed to parse latest version from GitHub API response"
    fi
}

# --- Main install function ---
install() {
    detect_downloader
    detect_target

    # Determine version
    VERSION="${REZ_NEXT_VERSION:-}"
    if [ -z "$VERSION" ] || [ "$VERSION" = "latest" ]; then
        info "Fetching latest version..."
        get_latest_version
    fi
    info "Installing ${BINARY_NAME} v${VERSION}..."

    # Determine install directory
    INSTALL_DIR="${REZ_NEXT_INSTALL:-$HOME/.rez-next/bin}"
    mkdir -p "$INSTALL_DIR"

    # Check for existing installation
    local existing_binary="$INSTALL_DIR/$BINARY_NAME"
    local old_version=""
    if [ -f "$existing_binary" ]; then
        old_version=$("$existing_binary" --version 2>/dev/null | head -1 | awk '{print $2}') || true
        if [ -n "$old_version" ]; then
            info "Found existing installation: ${BINARY_NAME} v${old_version}"
        fi
    fi

    # Create temp directory
    local tmpdir
    tmpdir=$(mktemp -d) || err "Failed to create temp directory"
    trap 'rm -rf "$tmpdir"' EXIT

    # Determine archive name and download URL
    local archive_name ext
    case "$OS" in
        windows)
            ext="zip"
            archive_name="${BINARY_NAME}-${TARGET}.zip"
            ;;
        *)
            ext="tar.gz"
            archive_name="${BINARY_NAME}-${TARGET}.tar.gz"
            ;;
    esac

    local download_url="https://github.com/${REPO}/releases/download/v${VERSION}/${archive_name}"
    local archive_path="$tmpdir/$archive_name"

    info "Downloading ${download_url}..."
    download "$download_url" "$archive_path" || err "Download failed. Check if v${VERSION} has pre-built binaries for ${TARGET}."

    # Verify SHA256 checksum if available
    local checksums_url="https://github.com/${REPO}/releases/download/v${VERSION}/checksums-sha256.txt"
    local checksums_path="$tmpdir/checksums-sha256.txt"
    if download "$checksums_url" "$checksums_path" 2>/dev/null; then
        info "Verifying SHA256 checksum..."
        local expected_hash actual_hash
        expected_hash=$(grep "$archive_name" "$checksums_path" | awk '{print $1}')
        if [ -n "$expected_hash" ]; then
            if command -v sha256sum >/dev/null 2>&1; then
                actual_hash=$(sha256sum "$archive_path" | awk '{print $1}')
            elif command -v shasum >/dev/null 2>&1; then
                actual_hash=$(shasum -a 256 "$archive_path" | awk '{print $1}')
            else
                warn "No SHA256 tool found, skipping checksum verification"
                actual_hash=""
            fi

            if [ -n "$actual_hash" ]; then
                if [ "$actual_hash" = "$expected_hash" ]; then
                    success "Checksum verified ✓"
                else
                    err "Checksum mismatch! Expected: ${expected_hash}, Got: ${actual_hash}"
                fi
            fi
        else
            warn "Checksum not found for ${archive_name}, skipping verification"
        fi
    else
        warn "Checksums file not available, skipping verification"
    fi

    # Extract
    info "Extracting..."
    case "$ext" in
        tar.gz)
            tar xzf "$archive_path" -C "$tmpdir"
            ;;
        zip)
            unzip -qo "$archive_path" -d "$tmpdir"
            ;;
    esac

    # Find and install binary
    local binary_path
    binary_path=$(find "$tmpdir" -name "$BINARY_NAME" -type f ! -name "*.txt" ! -name "*.tar.gz" ! -name "*.zip" | head -1)
    if [ -z "$binary_path" ]; then
        binary_path=$(find "$tmpdir" -name "${BINARY_NAME}.exe" -type f | head -1)
    fi
    if [ -z "$binary_path" ]; then
        err "Could not find ${BINARY_NAME} binary in the downloaded archive"
    fi

    chmod +x "$binary_path"
    mv "$binary_path" "$INSTALL_DIR/$BINARY_NAME"
    success "${BINARY_NAME} v${VERSION} installed to ${INSTALL_DIR}/${BINARY_NAME}"

    # Show upgrade info
    if [ -n "$old_version" ] && [ "$old_version" != "$VERSION" ]; then
        success "Upgraded: v${old_version} → v${VERSION}"
    fi

    # Check if install dir is in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *)
            echo ""
            warn "'${INSTALL_DIR}' is not in your PATH."
            echo ""
            echo "  To add it, run one of the following:"
            echo ""

            local shell_name
            shell_name=$(basename "${SHELL:-/bin/sh}")
            case "$shell_name" in
                zsh)
                    echo "    echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.zshrc"
                    echo "    source ~/.zshrc"
                    ;;
                bash)
                    echo "    echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.bashrc"
                    echo "    source ~/.bashrc"
                    ;;
                fish)
                    echo "    set -Ux fish_user_paths ${INSTALL_DIR} \$fish_user_paths"
                    ;;
                *)
                    echo "    export PATH=\"${INSTALL_DIR}:\$PATH\""
                    ;;
            esac
            echo ""
            ;;
    esac

    # Verify installation
    if "$INSTALL_DIR/$BINARY_NAME" --version >/dev/null 2>&1; then
        local installed_version
        installed_version=$("$INSTALL_DIR/$BINARY_NAME" --version 2>/dev/null | head -1)
        success "Verified: ${installed_version}"
    else
        warn "Could not verify installation, but binary was placed at ${INSTALL_DIR}/${BINARY_NAME}"
    fi

    echo ""
    info "Run '${BINARY_NAME} --help' to get started"
}

install
