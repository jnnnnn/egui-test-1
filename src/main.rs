#![warn(clippy::all, rust_2018_idioms)]
//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod icon;

use crate::icon::load_icon;
use std::path::PathBuf;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).

    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions {
        icon_data: load_icon(PathBuf::from(".").join("assets").join("icon-256.png")),
        initial_window_pos: Some(egui::Pos2::new(100.0, 100.0)),
        initial_window_size: Some(egui::Vec2::new(800.0, 600.0)),
        ..Default::default()
    };
    
    let result = eframe::run_native(
        "eframe template",
        native_options,

        Box::new(|cc| Box::new(rlgdesktop::TemplateApp::new(cc))),
    );
    if result.is_err() {
        println!("Error: {:?}", result);
    }
}

// when compiling to web using trunk.
#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();
    eframe::start_web(
        "the_canvas_id", // hardcode it
        web_options,
        Box::new(|cc| Box::new(rlgdesktop::TemplateApp::new(cc))),
    )
    .expect("failed to start eframe");
}
