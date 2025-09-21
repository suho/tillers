# Installation & Option-Key Migration Guide

TilleRS targets macOS power users who prefer keyboard-driven workflows and relies on the **Option
(⌥)** key as the primary modifier. This guide walks through installation, required permissions, and
migration from legacy Command-based shortcuts.

## Prerequisites

- macOS 12 (Monterey) or later
- Accessibility & Input Monitoring permissions (System Settings → Privacy & Security)
- Rust toolchain (1.75+) when building from source

## Install Options

### 1. Download release build

```bash
curl -L https://github.com/tillers/tillers/releases/latest/download/tillers-macos.tar.gz | tar xz
sudo cp tillers /usr/local/bin/
```

### 2. Build from source

```bash
git clone https://github.com/tillers/tillers
cd tillers
cargo build --release
cp target/release/tillers /usr/local/bin/
```

After installation, run the binary once so macOS prompts for Accessibility and Input Monitoring
permissions. Grant both permissions and relaunch the app.

## Create a macOS .app Bundle

Packaging the binary as `TilleRS.app` ensures macOS privacy prompts are associated with the bundle
and lets you pin the app in Launchpad or the Dock.

> **Shortcut**: run `scripts/package_app.sh --version <semver>` after each release build. The script
> performs every step below, signs the bundle with an ad-hoc identity (no Developer ID required),
> and clears the `com.apple.quarantine` attribute using the system `xattr` (or Homebrew's copy if
> available).

1. **Build a release binary**
   ```bash
   cargo build --release
   ```
   Keep the resulting binary at `target/release/tillers`.

2. **Create the bundle skeleton**
   ```bash
   APP=target/release/TilleRS.app
   mkdir -p "$APP/Contents/MacOS"
   mkdir -p "$APP/Contents/Resources"
   ```

3. **Copy bundle metadata and assets**
   ```bash
   cp resources/Info.plist "$APP/Contents/Info.plist"
   cp resources/entitlements.plist "$APP/Contents/Resources/entitlements.plist"
   ```
   Add icons or other assets to `Contents/Resources` as needed (e.g., `AppIcon.icns`).

4. **Install the executable**
   ```bash
   cp target/release/tillers "$APP/Contents/MacOS/TilleRS"
   chmod +x "$APP/Contents/MacOS/TilleRS"
   ```

5. **Code sign the bundle**
   - Sign the main executable with the desired entitlements:
     ```bash
     codesign \
       --force --options runtime \
       --entitlements "$APP/Contents/Resources/entitlements.plist" \
       --sign - "$APP/Contents/MacOS/TilleRS"
     ```
   - Then sign the container so macOS trusts the bundle structure:
     ```bash
     codesign --force --sign - "$APP"
     ```
   - Replace `-` with your Developer ID certificate name when you are ready to distribute.

6. **Verify and launch**
   ```bash
   codesign --verify --deep --strict --verbose=2 "$APP"
   open "$APP"
   ```
   macOS should now display the Accessibility, Input Monitoring, and Screen Recording prompts when
   needed.

7. **Optional: clear quarantine flags** (if the bundle was downloaded or copied from another
   machine). The script above does this automatically via `xattr -dr com.apple.quarantine`, but you
   can run it manually as well:
   ```bash
   xattr -cr "$APP"
   ```

With the bundle in place, drag `TilleRS.app` into `/Applications` or the Dock to keep it handy.

## Homebrew Cask Distribution

To publish `TilleRS.app` via a tap (e.g., `suho/homebrew-tap`), run:

```bash
scripts/create_cask_release.sh --version 0.3.0 \
  --download-url "https://github.com/suho/tillers/releases/download/v0.3.0/TilleRS-0.3.0.zip"
```

The script:
- builds/signs the bundle using `package_app.sh`
- produces `target/release/TilleRS-0.3.0.zip` (ad-hoc signed, quarantine cleared)
- writes a cask template to `target/release/tillers.cask.rb`

Copy the generated cask into `suho/homebrew-tap/Casks/tillers.rb`, adjust the URL if needed, and
push the tap. End users can then install with:

```bash
brew install --cask suho/tap/tillers
```

Because the bundle is not notarized, the first launch still requires the standard right-click →
Open confirmation even though Homebrew removes `com.apple.quarantine`.

## Option-Key Defaults

The project standardises on Option-based shortcuts to avoid conflicts with system Command shortcuts.
Common defaults:

| Action                       | Legacy Shortcut | New Shortcut |
|------------------------------|-----------------|--------------|
| Switch to workspace 1        | `cmd+1`         | `opt+1`      |
| Switch to workspace 2        | `cmd+2`         | `opt+2`      |
| Move window to workspace 1   | `cmd+shift+1`   | `opt+shift+1`|
| Cycle workspaces forward     | `cmd+tab`       | `opt+tab`    |
| Cycle workspaces backward    | `cmd+shift+tab` | `opt+shift+tab` |

These defaults are reflected in `~/.config/tillers/workspaces.toml`, `keybindings.toml`, and the
CLI help output.

## Migrating Existing Configurations

1. **Back up current config**
   ```bash
   cp -a ~/.config/tillers ~/.config/tillers.backup-$(date +%Y%m%d)
   ```
2. **Launch TilleRS once** – the configuration parser automatically converts legacy Command-based
   shortcuts to Option equivalents when loading files via `Workspace::migrate_legacy_command_shortcut`
   and `KeyboardMapping::migrate_command_to_option`.
3. **Review converted shortcuts** using ripgrep or your editor:
   ```bash
   rg "cmd\+" ~/.config/tillers
   ```
   The result set should be empty once migration finishes.
4. **Manual adjustments** – if you prefer hybrid modifiers (e.g., `opt+ctrl+1`), edit the relevant
   `keyboard_mappings` entries and restart TilleRS.

## Verifying the Migration

- Enable debug logging with `RUST_LOG=tillers=debug cargo run` (or for the installed binary,
  `RUST_LOG=tillers=debug tillers`). The workspace manager reports each shortcut migration at
  startup.
- Inspect the rendered configuration files after launch; shortcut strings should start with `opt+`
  once migration completes.

## Troubleshooting

- **Permission prompts reappear**: remove and re-add TilleRS in System Settings → Privacy & Security.
- **Shortcut conflicts remain**: inspect the macOS Keyboard Shortcuts panel for overlaps and adjust
  the Option-based mapping accordingly.
- **Migration skipped**: ensure configuration files are valid TOML. Invalid files are skipped and a
  warning is emitted through the CLI diagnostics channel.

Keeping the installation and migration steps documented ensures users understand the Option-key
transition and can confidently convert existing setups.
