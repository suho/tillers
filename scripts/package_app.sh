#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="$ROOT_DIR/target/release"
APP_NAME="TilleRS"
BINARY_NAME="tillers"
APP_PATH="$TARGET_DIR/${APP_NAME}.app"

VERSION=""
SKIP_BUILD="false"

print_help() {
    cat <<'EOF'
Usage: scripts/package_app.sh [options]

Options:
  --version <semver>   Update CFBundleShortVersionString and CFBundleVersion
  --skip-build         Reuse existing target/release build
  -h, --help           Show this help message

The script packages the tillers binary into TilleRS.app, signs it with an
ad-hoc signature, and removes the com.apple.quarantine attribute.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --skip-build)
            SKIP_BUILD="true"
            shift
            ;;
        -h|--help)
            print_help
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            print_help >&2
            exit 1
            ;;
    esac
done

if [[ "$SKIP_BUILD" != "true" ]]; then
    echo "Building release binary..."
    (cd "$ROOT_DIR" && cargo build --release)
else
    echo "Skipping cargo build (reuse existing artifacts)"
fi

if [[ ! -f "$TARGET_DIR/$BINARY_NAME" ]]; then
    echo "Error: binary $TARGET_DIR/$BINARY_NAME not found." >&2
    exit 1
fi

echo "Preparing bundle directories at $APP_PATH"
rm -rf "$APP_PATH"
mkdir -p "$APP_PATH/Contents/MacOS"
mkdir -p "$APP_PATH/Contents/Resources"

echo "Copying bundle metadata"
cp "$ROOT_DIR/resources/Info.plist" "$APP_PATH/Contents/Info.plist"
cp "$ROOT_DIR/resources/entitlements.plist" "$APP_PATH/Contents/Resources/entitlements.plist"

/usr/libexec/PlistBuddy -c "Set :CFBundleExecutable $APP_NAME" "$APP_PATH/Contents/Info.plist"

if [[ -n "$VERSION" ]]; then
    echo "Setting bundle version to $VERSION"
    /usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" "$APP_PATH/Contents/Info.plist"
    BUNDLE_BUILD=$(echo "$VERSION" | tr -c '0-9.' '.' | tr -s '.' '.' | sed 's/^\.//' | sed 's/\.$//')
    if [[ -z "$BUNDLE_BUILD" ]]; then
        BUNDLE_BUILD="0"
    fi
    /usr/libexec/PlistBuddy -c "Set :CFBundleVersion $BUNDLE_BUILD" "$APP_PATH/Contents/Info.plist"
fi

echo "Installing executable"
cp "$TARGET_DIR/$BINARY_NAME" "$APP_PATH/Contents/MacOS/$APP_NAME"
chmod +x "$APP_PATH/Contents/MacOS/$APP_NAME"

echo "Signing executable with ad-hoc identity"
codesign \
    --force --options runtime \
    --entitlements "$APP_PATH/Contents/Resources/entitlements.plist" \
    --sign - "$APP_PATH/Contents/MacOS/$APP_NAME"

echo "Signing bundle container"
codesign \
    --force \
    --sign - "$APP_PATH"

echo "Verifying signature"
codesign --verify --deep --strict --verbose=2 "$APP_PATH"

XATTR_BIN=""
if command -v xattr >/dev/null 2>&1; then
    XATTR_BIN="$(command -v xattr)"
elif command -v brew >/dev/null 2>&1; then
    PFX="$(brew --prefix)"
    if [[ -x "$PFX/bin/xattr" ]]; then
        XATTR_BIN="$PFX/bin/xattr"
    fi
fi

if [[ -n "$XATTR_BIN" ]]; then
    echo "Clearing com.apple.quarantine via $XATTR_BIN"
    "$XATTR_BIN" -dr com.apple.quarantine "$APP_PATH" || true
else
    echo "Warning: xattr command not found; skipping quarantine removal" >&2
fi

echo "Bundle ready at $APP_PATH"
echo "Launch with: open \"$APP_PATH\""
