#!/usr/bin/env bash
#
# Package the macOS app bundle into a zip suitable for Homebrew Cask and emit
# a cask formula template.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET_DIR="$ROOT_DIR/target/release"
APP_NAME="TilleRS"
APP_BUNDLE="$TARGET_DIR/${APP_NAME}.app"
ZIP_NAME=""
VERSION=""
DOWNLOAD_URL=""
SKIP_BUILD="false"

usage() {
    cat <<'EOF'
Usage: scripts/create_cask_release.sh --version <semver> [options]

Options:
  --version <semver>      Version string to embed in artifacts
  --download-url <url>    Public URL where the zip will be hosted (used in the
                          generated cask template). Defaults to a placeholder.
  --skip-build            Reuse existing target/release artifacts instead of
                          running cargo build
  -h, --help              Show this help message

The script builds the .app bundle (via package_app.sh), zips it for Homebrew
Cask, computes the SHA256 checksum, and emits a ready-to-edit cask formula.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --download-url)
            DOWNLOAD_URL="$2"
            shift 2
            ;;
        --skip-build)
            SKIP_BUILD="true"
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

if [[ -z "$VERSION" ]]; then
    echo "Error: --version is required" >&2
    usage >&2
    exit 1
fi

PACKAGE_ARGS=("--version" "$VERSION")
if [[ "$SKIP_BUILD" == "true" ]]; then
    PACKAGE_ARGS+=("--skip-build")
fi

echo "Packaging app bundle for version $VERSION"
"$ROOT_DIR/scripts/package_app.sh" "${PACKAGE_ARGS[@]}"

if [[ ! -d "$APP_BUNDLE" ]]; then
    echo "Error: app bundle not found at $APP_BUNDLE" >&2
    exit 1
fi

ZIP_NAME="${APP_NAME}-${VERSION}.zip"
ZIP_PATH="$TARGET_DIR/$ZIP_NAME"

echo "Creating zip archive at $ZIP_PATH"
rm -f "$ZIP_PATH"
ditto -c -k --sequesterRsrc --keepParent "$APP_BUNDLE" "$ZIP_PATH"

echo "Computing SHA256 checksum"
SHA256=$(shasum -a 256 "$ZIP_PATH" | awk '{print $1}')

if [[ -z "$DOWNLOAD_URL" ]]; then
    DOWNLOAD_URL="https://example.com/$ZIP_NAME"
fi

VERIFIED_URL=$(echo "$DOWNLOAD_URL" | sed -E 's|(https?://[^/]+).*|\1/|')

CASK_TEMPLATE="$TARGET_DIR/tillers.cask.rb"
cat >"$CASK_TEMPLATE" <<EOF
cask "tillers" do
  version "$VERSION"
  sha256 "$SHA256"

  url "$DOWNLOAD_URL",
      verified: "$VERIFIED_URL"
  name "TilleRS"
  desc "Keyboard-first tiling window manager for macOS"
  homepage "https://github.com/suho/tillers"

  app "${APP_NAME}.app"

  zap trash: [
    "~/.config/tillers",
  ]

  caveats <<~EOS
    TilleRS ships without Apple notarization. On first launch, right-click the app in Finder and choose "Open" to bypass Gatekeeper.
  EOS
end
EOF

cat <<EOF

Artifacts ready:
  - $ZIP_PATH
  - $CASK_TEMPLATE (update the URL if necessary)

Checksum:
  $SHA256

Next steps:
  1. Upload $ZIP_NAME to a publicly accessible location (e.g., GitHub Releases).
  2. Update $CASK_TEMPLATE with the final download URL if it differs.
  3. Commit the cask file to your tap (suho/homebrew-tap) under Casks/tillers.rb.
EOF
