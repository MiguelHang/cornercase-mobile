#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: scripts/ios-release.sh [--help]

Builds the iPhone app for the App Store and uploads it to App Store Connect, where it shows in TestFlight once
Apple has processed it. Run it on a Mac with Xcode, from any folder of the repository.

It reads the App Store Connect API key from the environment; the Tauri CLI signs with the same key:
  APPLE_API_KEY        the key's ID
  APPLE_API_ISSUER     the issuer ID of the key
  APPLE_API_KEY_PATH   the key's file, named AuthKey_<APPLE_API_KEY>.p8
  BUILD_NUMBER         optional, a whole number above the last upload's; by default the time, YYYYMMDDHHMM

On Xcode 27 the release build needs rustup's llvm-tools component (rustup component add llvm-tools); the
script refuses to start without it.

The build rewrites files of src-tauri/gen/apple that git tracks (ExportOptions.plist, Info.plist,
project.pbxproj, the scheme). The script restores that folder when it ends, even after a failure, so it refuses
to start while the folder has uncommitted changes. Should it be killed first, run
git checkout -- src-tauri/gen/apple
EOF
}

fail() {
    echo "ios-release: $1" >&2
    exit 1
}

need() {
    [ -n "${!1:-}" ] && return
    echo "ios-release: set $1 to $2" >&2
    missing=1
}

check_environment() {
    local missing=0
    need APPLE_API_KEY "the App Store Connect API key's ID"
    need APPLE_API_ISSUER "the issuer ID of the key"
    need APPLE_API_KEY_PATH "the key's .p8 file"
    [ "$missing" -eq 0 ] || fail "see scripts/ios-release.sh --help"
    [ -f "$APPLE_API_KEY_PATH" ] || fail "APPLE_API_KEY_PATH: no file at $APPLE_API_KEY_PATH"
    [ "$(basename "$APPLE_API_KEY_PATH")" = "AuthKey_$APPLE_API_KEY.p8" ] \
        || fail "APPLE_API_KEY_PATH: name the file AuthKey_$APPLE_API_KEY.p8, the name altool looks for"
}

check_llvm_tools() {
    rustup component list --installed | grep '^llvm-tools' > /dev/null \
        || fail "run rustup component add llvm-tools first: the release build needs it on Xcode 27"
}

case "${1:-}" in
    -h | --help) usage; exit 0 ;;
    "") ;;
    *) usage >&2; exit 2 ;;
esac

check_environment
build="${BUILD_NUMBER:-$(date +%Y%m%d%H%M)}"
[[ "$build" =~ ^[1-9][0-9]*$ ]] || fail "BUILD_NUMBER: $build is not a whole number above 0"
APPLE_API_KEY_PATH="$(cd "$(dirname "$APPLE_API_KEY_PATH")" && pwd)/$(basename "$APPLE_API_KEY_PATH")"
export APPLE_API_KEY APPLE_API_ISSUER APPLE_API_KEY_PATH

cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
check_llvm_tools
[ -z "$(git status --porcelain -- src-tauri/gen/apple)" ] \
    || fail "commit or stash the changes in src-tauri/gen/apple first: the script restores that folder when it ends"

marker="$(mktemp)"
trap 'git checkout -- src-tauri/gen/apple; rm -f "$marker"' EXIT

LANG=en_US.UTF-8 pnpm tauri ios build --export-method app-store-connect --ci \
    --config "{\"bundle\":{\"iOS\":{\"bundleVersion\":\"$build\"}}}"

ipa="$(find src-tauri/gen/apple/build -name '*.ipa' -newer "$marker")"
if [ -z "$ipa" ] || [ "$(printf '%s\n' "$ipa" | wc -l)" -ne 1 ]; then
    fail "expected one new .ipa in src-tauri/gen/apple/build, found: ${ipa:-none}"
fi

API_PRIVATE_KEYS_DIR="$(dirname "$APPLE_API_KEY_PATH")" xcrun altool --upload-app -f "$ipa" -t ios \
    --api-key "$APPLE_API_KEY" --api-issuer "$APPLE_API_ISSUER"
echo "ios-release: uploaded $ipa as build $build"
