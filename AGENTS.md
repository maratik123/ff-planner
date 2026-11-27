# AGENTS.md

Rust + egui/eframe desktop and WASM app ("ff-planner", a settlement/layout planner). Not a workspace: single crate `ff_planner` in `src/` (`main.rs` native/WASM entry, `lib.rs` -> `app.rs` `TemplateApp`).

## Commands
- Dev: `cargo run --release` (native); web: `trunk serve` (URL `http://127.0.0.1:8080/index.html#dev`; `#dev` disables service-worker caching).
- Full local verification (CI-like): `./check.sh` (cargo check, wasm32 check, fmt, clippy `-D warnings`, tests, `trunk build`).
- Single test: `cargo test --workspace --lib` (no integration tests currently).
- Toolchain pinned in `rust-toolchain` (rustfmt/clippy, wasm32 target). Do not bypass it with `rustup default`.

## Architecture / conventions
- Dependency versions in `Cargo.toml` follow semver-minimized rules: `0.x.y` is written as `0.x` (e.g. `env_logger = "0.11"`), and `x.y.z` is written as `x` (e.g. `serde = { version = "1", ... }`). The package's own `version` and `rust-version` are exempt.
- Native and WASM share one codebase via `cfg(target_arch = "wasm32")` branches in `main.rs`; keep platform-specific code behind those cfgs.
- egui app state is serde-persisted via eframe `persistence` (`cc.storage`, `APP_KEY`); fields added to the app struct should be `#[serde]`-compatible (defaults) so old saved state still loads.
- `index.html` has hard-coded element ids (`the_canvas_id`, `loading_text`) that `src/main.rs` expects — keep them in sync.
- Lints are strict: `Cargo.toml` workspace lints (`clippy::all` + `pedantic`, many extra warns, `unsafe_code = deny`). CI sets `RUSTFLAGS=-D warnings` and `RUSTDOCFLAGS=-D warnings`; `check.sh` denies warnings in clippy. Follow the existing style; don't silence lints casually.
- Test Driven Development.
- Public API: only the entry points consumers actually call (e.g. `all()`) plus the types (recursively, including fields) they return. Everything else is private.
- No `pub` on helpers, fields, or modules that consumers don't need.
- No assert/panic in production code except `expect` where a failure is a compile-time-known artifact; all checks live in tests.
- Comments: keep only why-comments that the code can't express; never re-state the code; minimize them.
- Paths to embedded data: build with `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/..."))`, not relative literals.

## CI
`.github/workflows/rust.yml`: cargo check (native + wasm32), fmt, clippy, `cargo test --lib`, `trunk build`, release builds for macOS/Windows/Linux(musl via cross). `pages.yml` deploys `trunk build --release --public-url ...` to GitHub Pages on main.

## Git workflow
- Commit yourself as you work — do not wait for owner permission to commit.
- Never push without explicit owner permission; pushing is the action that requires owner approval.

## Planning docs
- `docs/README.md` — индекс планирования разработки; перед началом работы зачитай этот файл и выбери подзадачу, не трогая чужие фазы.
