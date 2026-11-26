# AGENTS.md

Rust + egui/eframe desktop and WASM app ("ff-planner", a settlement/layout planner). Not a workspace: single crate `ff_planner` in `src/` (`main.rs` native/WASM entry, `lib.rs` -> `app.rs` `TemplateApp`).

## Commands
- Dev: `cargo run --release` (native); web: `trunk serve` (URL `http://127.0.0.1:8080/index.html#dev`; `#dev` disables service-worker caching).
- Full local verification (CI-like): `./check.sh` (cargo check, wasm32 check, fmt, clippy `-D warnings`, tests, `trunk build`).
- Single test: `cargo test --workspace --lib` (no integration tests currently).
- Toolchain pinned in `rust-toolchain` (1.100.0, rustfmt/clippy, wasm32 target). Do not bypass it with `rustup default`.

## Architecture / conventions
- Native and WASM share one codebase via `cfg(target_arch = "wasm32")` branches in `main.rs`; keep platform-specific code behind those cfgs.
- egui app state is serde-persisted via eframe `persistence` (`cc.storage`, `APP_KEY`); fields added to the app struct should be `#[serde]`-compatible (defaults) so old saved state still loads.
- `index.html` has hard-coded element ids (`the_canvas_id`, `loading_text`) that `src/main.rs` expects — keep them in sync.
- Lints are strict: `Cargo.toml` workspace lints (`clippy::all` + `pedantic`, many extra warns, `unsafe_code = deny`). CI sets `RUSTFLAGS=-D warnings` and `RUSTDOCFLAGS=-D warnings`; `check.sh` denies warnings in clippy. Follow the existing style; don't silence lints casually.
- Linux build needs system packages (see README: libxcb/libxkbcommon/libssl etc.); Nix flake provides a devShell with them.

## CI
`.github/workflows/rust.yml`: cargo check (native + wasm32), fmt, clippy, `cargo test --lib`, `trunk build`, release builds for macOS/Windows/Linux(musl via cross). `pages.yml` deploys `trunk build --release --public-url ...` to GitHub Pages on main. `typos.yml` runs typos (wordlist in `.typos.toml`).

## Repo state
Currently a scaffolded egui template (`TemplateApp` in `src/app.rs` is demo content). The real planner domain code is not yet present; expect to add modules under `src/` — keep the native/WASM dual-target and persistence patterns.
