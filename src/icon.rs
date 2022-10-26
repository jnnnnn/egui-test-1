use std::path::PathBuf;

pub fn load_icon(path: PathBuf) -> Option<eframe::IconData> {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path).ok()?.into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    Some(eframe::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    })
}
