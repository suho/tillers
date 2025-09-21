# API Documentation with rustdoc

TilleRS exposes a large set of public types across `models`, `services`, and `macos` integration layers.
Generate reference documentation with [`rustdoc`](https://doc.rust-lang.org/rustdoc/) to keep
those APIs discoverable while you iterate on the project.

## Quick start

```bash
# Ensure the compile-time rustc metadata is available for env!("RUSTC_VERSION")
RUSTC_VERSION="$(rustc --version)" \
  cargo doc --no-deps --package tillers
```

This command builds documentation for the public API surface without pulling in documentation for
all dependencies. The output lands in `target/doc/tillers/index.html`.

To preview the documentation locally:

```bash
open target/doc/tillers/index.html
# or
python3 -m http.server --directory target/doc
```

## Recommended workflow

1. Add module- or item-level doc comments as you introduce new public types.
2. Run `cargo doc --no-deps` locally to verify the documentation compiles.
3. Treat warnings as actionable; `RUSTDOCFLAGS="-D warnings"` can help catch missing docs early.
4. When updating keyboard shortcut behaviour, highlight the Option-key defaults in the relevant
   service or model docs so the change is visible in generated output.
5. Publish the rendered documentation as part of release artifacts when preparing macOS builds.

## Troubleshooting

- **`RUSTC_VERSION` missing**: The CLI currently embeds the toolchain version via `env!("RUSTC_VERSION")`.
  Export the variable as shown above or update the code to fall back to a runtime lookup.
- **Downstream compile errors**: rustdoc builds the entire crate graph; if other modules contain
  WIP code, temporarily gate them behind `#[cfg(doc)]` or stub out dependencies so documentation can build.
- **Slow rebuilds**: Use `cargo doc --open` for iterative development; combine with `RUSTDOCFLAGS="--cfg docsrs"`
  if documenting `cfg_attr` or docs.rs-specific content.

Keeping rustdoc up to date ensures the macOS accessibility integrations and Option-key keyboard
migration rules remain well documented for contributors.
