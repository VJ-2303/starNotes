# projectmem - starNotes

_Last updated: 2026-11-01_

## Project purpose
StarNote is a native note-taking app for Linux built with Rust and [GPUI Kit](https://github.com/longbridge/gpui-kit).

## Recent issues
- [DONE] #0001 blaze fails on freetype-sys build script: unsupported writable static __RUST_STD_INTERNAL_VAL [freetype-sys v0.20.1] -> Use cargo with sccache and mold linker instead of blaze to avoid ra-jit freetype-sys static failure and stay within 7GB RAM [.cargo/config.toml] (fixed)
- [DONE] #legacy_41c8 Legacy issue: fix(entity): add missing SeaORM entities for boards and entries -> fix(entity): add missing SeaORM entities for boards and entries (fixed)

## Decisions
- Configure cargo with 3 parallel jobs, sccache compiler cache, and mold linker to fit within 7GB RAM limit [.cargo/config.toml]
- Refactored application into pure note-taking by removing dead Kanban, Board, Card, and List code, models, and UI references [crates/]
- Removed Vim emulation mode and associated keymaps, overlays, state, and settings [crates/document_editor/]
- Switched global allocator to mimalloc to reduce memory fragmentation and lower RSS on Linux [crates/app/src/main.rs]
- Rename application from Castle to StarNote across binary, desktop entries, app paths, onboarding, and docs

## Notes
- Initial commit
- release: v0.4.0
- chore(release): v0.4.0 clean build and desktop installation
- ci: add libxkbcommon-x11-dev and xcb dev headers to release workflow
- gotcha: blaze cannot compile C-binding build scripts like freetype-sys with cc jobserver, plus consumes 4GB+ RAM [freetype-sys]
- gotcha: cargo test builds with separate profile from cargo check; use cargo check --tests for fast iterative verification without long re-compilation [Cargo.toml]
- gotcha: tree-sitter cannot be updated to 0.27 because gpui-kit pins tree-sitter ^0.26.13; gpui-kit 0.8.1 removed manual Root::render_dialog_layer in favor of automatic RootPlugin overlays

## Key files
- `.codex/config.toml`
- `.github/workflows/release.yml`
- `.gitignore`
- `.rustfmt.toml`
- `AGENTS.md`
- `Cargo.lock`
- `Cargo.toml`
- `Makefile`
- `README.md`
- `RELEASING.md`
- `crates/app/Cargo.toml`
- `crates/app/assets/fonts/ibm-plex-mono/IBMPlexMono-Bold.ttf`
- `crates/app/assets/fonts/ibm-plex-mono/IBMPlexMono-BoldItalic.ttf`
- `crates/app/assets/fonts/ibm-plex-mono/IBMPlexMono-Italic.ttf`
- `crates/app/assets/fonts/ibm-plex-mono/IBMPlexMono-Regular.ttf`
- `crates/app/assets/fonts/ibm-plex-mono/LICENSE.txt`
- `crates/app/assets/fonts/ibm-plex-sans/IBMPlexSans-Bold.ttf`
- `crates/app/assets/fonts/ibm-plex-sans/IBMPlexSans-BoldItalic.ttf`
- `crates/app/assets/fonts/ibm-plex-sans/IBMPlexSans-Italic.ttf`
- `crates/app/assets/fonts/ibm-plex-sans/IBMPlexSans-Medium.ttf`

## Open questions
- None logged yet.
