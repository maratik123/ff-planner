#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub mod buildings;
mod canvas;
mod frame_cache;
pub mod history;
mod panels;
pub mod placement;
mod selection;
pub mod validation;
pub use app::TemplateApp;
