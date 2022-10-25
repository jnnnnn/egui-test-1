use std::collections::HashMap;

use egui_extras::{Size, TableBuilder};

use crate::db;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    label: String,
    db_path: String,
    #[serde(skip)] // don't persist this
    // map key -> query
    filters: HashMap<String, String>,
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
            label: "Hello World!".to_owned(),
            db_path: "".to_owned(),
            value: 2.7,
            filters: HashMap::new(),
            db: None,
            results: Err(String::from("No results")),
        }
    }
}

struct ColumnConfig {
    width: f32,
    label: &'static str,
}

impl ColumnConfig {
    const fn new(label: &'static str) -> Self {
        Self {
            width: 100.0,
            label,
        }
    }
    const fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
}

const COLUMNS: &[ColumnConfig] = &[
    ColumnConfig::new("Title").width(300.0),
    ColumnConfig::new("Authors"),
    ColumnConfig::new("Series"),
    ColumnConfig::new("Year"),
    ColumnConfig::new("Language"),
    ColumnConfig::new("Publisher"),
    ColumnConfig::new("FileSize"),
    ColumnConfig::new("Format"),
    ColumnConfig::new("Download"),
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
            label: query,
            db_path,
            value: _,
            filters,
            db,
            results,
        } = self;

        if let Some(db) = db {
            match db.get_result() {
                Some(Ok(mut newbooks)) => match results {
                    Ok(bookcache) => {
                        while newbooks.len() > 0 {
                            bookcache.push(newbooks.pop().unwrap());
                        }
                    }
                    Err(_) => {
                        *results = Ok(newbooks);
                    }
                },
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

            ui.horizontal(|ui| {
                ui.label("Query: ");
                if ui.text_edit_singleline(query).lost_focus() {
                    if let Some(db) = db {
                        db.interrupt();
                        db.query(query);
                        *results = Err(String::from("Searching..."));
                    }
                }
            });
            render_filter(ui, String::from("Language"), filters)
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

fn render_filter(ui: &mut egui::Ui, label: String, filters: &mut HashMap<String, String>) {
    ui.horizontal(|ui| {
        ui.label(label.to_owned());
        let mut query = filters.get(&label).unwrap_or(&String::new()).to_owned();
        let e = ui.text_edit_singleline(&mut query);
        if e.changed() {
            filters.insert(label.to_owned(), query);
        }
    });
}

fn render_results_table(ui: &mut egui::Ui, books: &Vec<db::Book>) {
    let mut tb = TableBuilder::new(ui);
    for col in COLUMNS.iter() {
        tb = tb.column(Size::exact(col.width));
    }
    tb.header(20.0, |mut header| {
        for col in COLUMNS.iter() {
            header.col(|ui| {
                ui.label(col.label);
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
                format!("{:.2}", books[i].sizeinbytes as f32 / 1024.0).as_str(),
            );
            render_text_cell(&mut row, books[i].format.as_str());
            render_text_cell(&mut row, books[i].locator.as_str());
        });
    });
}

fn render_text_cell(row: &mut egui_extras::TableRow, text: &str) {
    row.col(|ui| {
        ui.label(text);
    });
}
