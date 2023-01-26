#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod db;
mod download;
pub use app::TemplateApp;
mod uifilter;
mod config;