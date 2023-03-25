# rlgdesktop egui test

## Testing locally

Make sure you are using the latest version of stable rust by running `rustup update`. 

Get a copy of the index database (see devlog).

To start a local instance, run:

```sh
cp Settings.yaml.example.yaml Settings.yaml # and edit as appropriate
cargo install cargo-watch
cargo watch -x run  
```

## Updating egui

As of 2022, egui is in active development with frequent releases with breaking changes. [eframe_template](https://github.com/emilk/eframe_template/) will be updated in lock-step to always use the latest version of egui.

When updating `egui` and `eframe` it is recommended you do so one version at the time, and read about the changes in [the egui changelog](https://github.com/emilk/egui/blob/master/CHANGELOG.md) and [eframe changelog](https://github.com/emilk/egui/blob/master/crates/eframe/CHANGELOG.md).

### Learning about egui

`src/app.rs` contains a simple example app.

The official egui docs are at <https://docs.rs/egui>. If you prefer watching a video introduction, check out <https://www.youtube.com/watch?v=NtUkr_z7l84>. For inspiration, check out the [the egui web demo](https://emilk.github.io/egui/index.html) and follow the links in it to its source code.

### To compile for mac M1/arm on a windows machine

```sh
rustup target add aarch64-apple-darwin
cargo build --target aarch64-apple-darwin
```
