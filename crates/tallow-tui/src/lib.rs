//! Tallow's frontend: the app, its screens and the log. Shared by the
//! terminal binary (`main.rs`) and the browser build (`tallow-web`).

pub mod app;
pub mod input;
pub mod journal;
mod log;
mod names;
pub mod render;
pub mod save;

pub use app::App;
