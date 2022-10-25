use std::sync::atomic::Ordering::Relaxed;

use egui_extras::{Size, TableBuilder};

use crate::db;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    db_path: String,
    #[serde(skip)]
    filters: db::Params,
    #[serde(skip)]
    value: f32,
    #[serde(skip)]
    db: Option<db::DB>,
    #[serde(skip)]
    results: Result<Vec<db::Book>, String>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            db_path: "".to_owned(),
            value: 2.7,
            filters: db::Params::default(),
            db: None,
            results: Err(String::from("No results")),
        }
    }
}

const COLUMNS: &'static [&'static str] = &[
    "Title",
    "Authors",
    "Series",
    "Year",
    "Language",
    "Publisher",
    "FileSize",
    "Format",
    "Download",
];

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Default::default()
    }
}

impl eframe::App for TemplateApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            db_path,
            value: _,
            filters,
            db,
            results,
        } = self;

        if let Some(db) = db {
            match db.get_result() {
                Some(Ok(newbooks)) => read_results(results, newbooks, ctx),
                Some(Err(e)) => *results = Err(e.to_string()),
                None => {}
            }
        } else if !db_path.is_empty() {
            *db = Some(db::DB::new(db_path));
        }

        // For inspiration and more examples, go to https://emilk.github.io/egui
        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Side Panel");

            ui.horizontal(|ui| {
                ui.label("DB Path: ");
                if ui.text_edit_singleline(db_path).changed() {
                    *db = Some(db::DB::new(db_path));
                }
            });

            let mut changed = true;
            let f = filters.collection == "fiction";
            if ui.selectable_label(f, "Fiction").clicked() {
                filters.collection = "fiction".to_owned();
            } else if ui.selectable_label(!f, "Nonfiction").clicked() {
                filters.collection = "nonfiction".to_owned();
            } else {
                changed = false;
            }

            changed |= render_filter(ui, "Title", &mut filters.title);
            changed |= render_filter(ui, "Authors", &mut filters.authors);
            changed |= render_filter(ui, "Series", &mut filters.series);
            changed |= render_filter(ui, "Language", &mut filters.language);
            changed |= render_filter(ui, "Format", &mut filters.format);

            if changed {
                if let Some(db) = db {
                    db.interrupt();
                    while db.get_result().is_some() {}
                    db.query(filters.clone());
                    *results = Err(String::from("Searching..."));
                }
            }

            if let Some(db) = db {
                if db.processing.load(Relaxed) {
                    ui.spinner();
                }
                ui.label(format!(
                    "{} results",
                    match results {
                        Ok(v) => v.len(),
                        _ => 0,
                    }
                ));
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| match results {
            Ok(books) => render_results_table(ui, books),
            Err(e) => {
                ui.label(e.to_string());
                ()
            }
        });
    }
}

fn read_results(
    results: &mut Result<Vec<db::Book>, String>,
    mut newbooks: Vec<db::Book>,
    ctx: &egui::Context,
) {
    if let Ok(bookcache) = results {
        while newbooks.len() > 0 {
            bookcache.push(newbooks.pop().unwrap());
            // keep redrawing while results are available even if mouse is not moving
            ctx.request_repaint();
        }
    } else {
        // no previous results, just replace the previous error
        *results = Ok(newbooks);
    }
}

fn render_filter(ui: &mut egui::Ui, label: &str, text: &mut String) -> bool {
    let mut result = false;
    ui.horizontal(|ui| {
        ui.label(label.to_owned());
        let e = ui.text_edit_singleline(text);
        result = e.changed();
    });
    return result;
}

fn render_results_table(ui: &mut egui::Ui, books: &Vec<db::Book>) {
    let mut tb = TableBuilder::new(ui);
    for col in COLUMNS.iter() {
        match *col {
            "Title" => {
                tb = tb.column(Size::Remainder {
                    range: (100.0, 3000.0),
                })
            }
            "Year" | "Language" | "FileSize" | "Format" | "Publisher" => {
                tb = tb.column(Size::Relative {
                    fraction: 0.05,
                    range: (20.0, 100.0),
                })
            }
            _ => {
                tb = tb.column(Size::Relative {
                    fraction: 0.15,
                    range: (100.0, 1000.0),
                })
            }
        };
    }
    tb.header(20.0, |mut header| {
        for col in COLUMNS.iter() {
            header.col(|ui| {
                ui.label(*col);
            });
        }
    })
    .body(|body| {
        body.rows(30.0, books.len(), |i, mut row| {
            render_text_cell(&mut row, books[i].title.as_str());
            render_text_cell(&mut row, books[i].authors.as_str());
            render_text_cell(&mut row, books[i].series.as_str());
            render_text_cell(&mut row, books[i].year.as_str());
            render_text_cell(&mut row, books[i].language.as_str());
            render_text_cell(&mut row, books[i].publisher.as_str());
            render_text_cell(
                &mut row,
                format!("{:.0}", books[i].sizeinbytes as f32 / 1024.0).as_str(),
            );
            render_text_cell(&mut row, books[i].format.as_str());
            render_text_cell(&mut row, books[i].locator.as_str());
        });
    });
}

fn render_text_cell(row: &mut egui_extras::TableRow<'_, '_>, text: &str) {
    row.col(|ui| {
        ui.label(text);
    });
}
