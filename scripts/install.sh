#!/bin/sh
set -e

# This script installs 'baproto' by downloading prebuilt binaries from the
# project's GitHub releases page. By default the latest version is installed,
# but a different release can be used instead by setting $BAPROTO_VERSION.
#
# The script will set up a 'baproto' cache at '%LOCALAPPDATA%/baproto'. This
# behavior can be customized by setting '$BAPROTO_HOME' prior to running the
# script. Note that this script will overwrite any 'baproto' binary artifacts
# in '$BAPROTO_HOME/bin'.

# ------------------------------ Define: Cleanup ----------------------------- #

trap cleanup EXIT

cleanup() {
    if [ -d "${BAPROTO_TMP=}" ]; then
        rm -rf "${BAPROTO_TMP}"
    fi
}

# ------------------------------ Define: Logging ----------------------------- #

info() {
    if [ "$1" != "" ]; then
        echo info: "$@"
    fi
}

warn() {
    if [ "$1" != "" ]; then
        echo warning: "$1"
    fi
}

error() {
    if [ "$1" != "" ]; then
        echo error: "$1" >&2
    fi
}

fatal() {
    error "$1"
    exit 1
}

unsupported_platform() {
    error "$1"
    echo "See https://github.com/coffeebeats/baproto/blob/main/docs/installation.md#install-from-source for instructions on compiling from source."
    exit 1
}

# ------------------------------- Define: Usage ------------------------------ #

usage() {
    cat <<EOF
baproto-install: Install 'baproto' for compiling custom binary encodings into language-specific bindings.

Usage: baproto-install [OPTIONS]

NOTE: The following dependencies are required:
    - curl OR wget
    - grep
    - sha256sum OR shasum
    - tar/unzip
    - tr
    - uname

Available options:
    -h, --help          Print this help and exit
    -v, --verbose       Print script debug info (default=false)
    --no-modify-path    Do not modify the \$PATH environment variable
EOF
    exit
}

check_cmd() {
    command -v "$1" >/dev/null 2>&1
}

need_cmd() {
    if ! check_cmd "$1"; then
        fatal "required command not found: '$1'"
    fi
}

# ------------------------------ Define: Params ------------------------------ #

parse_params() {
    MODIFY_PATH=1

    while :; do
        case "${1:-}" in
        -h | --help) usage ;;
        -v | --verbose) set -x ;;

        --no-modify-path) MODIFY_PATH=0 ;;

        -?*) fatal "Unknown option: $1" ;;
        "") break ;;
        esac
        shift
    done

    return 0
}

parse_params "$@"

# ------------------------------ Define: Version ----------------------------- #

BAPROTO_VERSION="${BAPROTO_VERSION:-0.2.4}" # x-release-please-version
BAPROTO_VERSION="v${BAPROTO_VERSION#v}"

# ----------------------------- Define: Platform ----------------------------- #

need_cmd tr
need_cmd uname

BAPROTO_OS="$(echo "${BAPROTO_OS=$(uname -s)}" | tr '[:upper:]' '[:lower:]')"
case "$BAPROTO_OS" in
darwin*) BAPROTO_OS="macos" ;;
linux*) BAPROTO_OS="linux" ;;
mac | macos | osx) BAPROTO_OS="macos" ;;
cygwin*) BAPROTO_OS="windows" ;;
msys* | mingw64*) BAPROTO_OS="windows" ;;
uwin* | win*) BAPROTO_OS="windows" ;;
*) unsupported_platform "no prebuilt binaries available for operating system: $BAPROTO_OS" ;;
esac

BAPROTO_ARCH="$(echo ${BAPROTO_ARCH=$(uname -m)} | tr '[:upper:]' '[:lower:]')"
case "$BAPROTO_ARCH" in
aarch64 | arm64)
    BAPROTO_ARCH="arm64"
    if [ "$BAPROTO_OS" != "macos" ] && [ "$BAPROTO_OS" != "linux" ]; then
        fatal "no prebuilt '$BAPROTO_ARCH' binaries available for operating system: $BAPROTO_OS"
    fi

    ;;
amd64 | x86_64) BAPROTO_ARCH="x86_64" ;;
*) unsupported_platform "no prebuilt binaries available for CPU architecture: $BAPROTO_ARCH" ;;
esac

BAPROTO_ARCHIVE_EXT=""
case "$BAPROTO_OS" in
windows) BAPROTO_ARCHIVE_EXT="zip" ;;
*) BAPROTO_ARCHIVE_EXT="tar.gz" ;;
esac

BAPROTO_ARCHIVE="baproto-$BAPROTO_VERSION-$BAPROTO_OS-$BAPROTO_ARCH.$BAPROTO_ARCHIVE_EXT"

# ------------------------------- Define: Store ------------------------------ #

BAPROTO_HOME_PREV="${BAPROTO_HOME_PREV=}" # save for later in script

BAPROTO_HOME="${BAPROTO_HOME=}"
if [ "$BAPROTO_HOME" = "" ]; then
    if [ "${HOME=}" = "" ]; then
        fatal "both '\$BAPROTO_HOME' and '\$HOME' unset; one must be specified to determine 'baproto' installation path"
    fi

    BAPROTO_HOME="$HOME/.baproto"
fi

info "using 'baproto' store path: '$BAPROTO_HOME'"

# ----------------------------- Define: Download ----------------------------- #

need_cmd grep
need_cmd mktemp

BAPROTO_TMP=$(mktemp -d --tmpdir baproto-XXXXXXXXXX)
cd "$BAPROTO_TMP"

BAPROTO_RELEASE_URL="https://github.com/coffeebeats/build-a-proto/releases/download/$BAPROTO_VERSION"

download_with_curl() {
    curl \
        --fail \
        --location \
        --parallel \
        --retry 3 \
        --retry-delay 1 \
        --show-error \
        --silent \
        -o "$BAPROTO_ARCHIVE" \
        "$BAPROTO_RELEASE_URL/$BAPROTO_ARCHIVE" \
        -o "checksums.txt" \
        "$BAPROTO_RELEASE_URL/checksums.txt"
}

download_with_wget() {
    wget -q -t 4 -O "$BAPROTO_ARCHIVE" "$BAPROTO_RELEASE_URL/$BAPROTO_ARCHIVE" 2>&1
    wget -q -t 4 -O "checksums.txt" "$BAPROTO_RELEASE_URL/checksums.txt" 2>&1
}

if check_cmd curl; then
    download_with_curl
elif check_cmd wget; then
    download_with_wget
else
    fatal "missing one of 'curl' or 'wget' commands"
fi

# -------------------------- Define: Verify checksum ------------------------- #

verify_with_sha256sum() {
    cat "checksums.txt" | grep "$BAPROTO_ARCHIVE" | sha256sum --check --status
}

verify_with_shasum() {
    cat "checksums.txt" | grep "$BAPROTO_ARCHIVE" | shasum -a 256 -p --check --status
}

if check_cmd sha256sum; then
    verify_with_sha256sum
elif check_cmd shasum; then
    verify_with_shasum
else
    fatal "missing one of 'sha256sum' or 'shasum' commands"
fi

# ------------------------------ Define: Extract ----------------------------- #

case "$BAPROTO_OS" in
windows)
    need_cmd unzip

    mkdir -p "$BAPROTO_HOME/bin"
    unzip -u "$BAPROTO_ARCHIVE" -d "$BAPROTO_HOME/bin"
    ;;
*)
    need_cmd tar

    mkdir -p "$BAPROTO_HOME/bin"
    tar -C "$BAPROTO_HOME/bin" --no-same-owner -xzf "$BAPROTO_ARCHIVE"
    ;;
esac

info "successfully installed 'baproto@$BAPROTO_VERSION' to '$BAPROTO_HOME/bin'"

if [ $MODIFY_PATH -eq 0 ]; then
    exit 0
fi

# The $PATH modification and $BAPROTO_HOME export is already done.
if check_cmd baproto && [ "$BAPROTO_HOME_PREV" != "" ]; then
    exit 0
fi

# Simplify the exported $BAPROTO_HOME if possible.
if [ "$HOME" != "" ]; then
    case "$BAPROTO_HOME" in
    $HOME*) BAPROTO_HOME="\$HOME${BAPROTO_HOME#$HOME}" ;;
    esac
fi

CMD_EXPORT_HOME="export BAPROTO_HOME=\"$BAPROTO_HOME\""
CMD_MODIFY_PATH="export PATH=\"\$BAPROTO_HOME/bin:\$PATH\""

case $(basename $SHELL) in
sh) OUT="$HOME/.profile" ;;
bash) OUT="$HOME/.bashrc" ;;
zsh) OUT="$HOME/.zshrc" ;;
*)
    echo ""
    echo "Add the following to your shell profile script:"
    echo "    $CMD_EXPORT_HOME"
    echo "    $CMD_MODIFY_PATH"
    ;;
esac

if [ "$OUT" != "" ]; then
    if [ -f "$OUT" ] && $(cat "$OUT" | grep -q 'export BAPROTO_HOME'); then
        info "Found 'BAPROTO_HOME' export in shell Rc file; skipping modification."
        exit 0
    fi

    if [ -f "$OUT" ] && [ "$(tail -n 1 "$OUT")" != "" ]; then
        echo "" >>"$OUT"
    fi

    echo "# Added by 'baproto' install script." >>"$OUT"
    echo "$CMD_EXPORT_HOME" >>"$OUT"
    echo "$CMD_MODIFY_PATH" >>"$OUT"

    info "Updated shell Rc file: $OUT\n      Open a new terminal to start using 'baproto'."
fi
