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
