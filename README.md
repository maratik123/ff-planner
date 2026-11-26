# ff-planner

Планировщик поселений для игры [Farthest Frontier](https://steamcommunity.com/app/1044720):
просмотр, редактор и генератор layout-ов на Rust.

Приложение построено на [egui](https://github.com/emilk/egui) / [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) и может
запускаться нативно (Linux/Windows/macOS) или как веб-приложение (WASM).

## Сборка и запуск

Требуется Rust (см. `rust-toolchain`, канал 1.100.0).

```sh
cargo run --release
```

На Linux перед сборкой понадобятся системные пакеты:

```sh
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

На Fedora:

```sh
dnf install clang clang-devel clang-tools-extra libxkbcommon-devel pkg-config openssl-devel libxcb-devel gtk3-devel atk fontconfig-devel
```

## Веб-версия (WASM)

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
trunk serve
```

Приложение откроется на `http://127.0.0.1:8080/index.html#dev` (флаг `#dev` отключает кэширование service worker).
Для публикации: `trunk build --release` — результат окажется в каталоге `dist`.

## Проверки

`check.sh` выполняет CI-подобные проверки: `cargo check`, `cargo fmt --check`, `cargo clippy`, `cargo test`, `trunk build` и проверку таргета `wasm32-unknown-unknown`.

## Лицензия

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
