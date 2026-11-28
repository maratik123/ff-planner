#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub mod buildings;
mod canvas;
pub mod history;
mod panels;
pub mod placement;
mod selection;
pub mod validation;
pub use app::TemplateApp;
