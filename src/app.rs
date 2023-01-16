use std::{cmp::Ordering, ops::RangeInclusive, sync::atomic::Ordering::Relaxed};

use egui_extras::{Column, TableBuilder};

use crate::{
    db::{
        self,
        Collection::{Fiction, NonFiction},
    },
    download,
    uifilter::{filter_update_booklist, UIFilter},
};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    db_path: String,
    filters: db::Params,
    #[serde(skip)]
    value: f32,
    #[serde(skip)]
    db: Option<db::DB>,
    #[serde(skip)]
    download: download::Download,
    #[serde(skip)]
    download_status: download::Status,
    #[serde(skip)]
    results: Result<Vec<db::Book>, String>,
    #[serde(skip)]
    uifilter: UIFilter,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            db_path: "".to_owned(),
            value: 2.7,
            filters: db::Params::default(),
            db: None,
            results: Err(String::from("No results")),
            download: download::Download::new(),
            download_status: download::Status::default(),
            uifilter: UIFilter::default(),
        }
    }
}

const COLUMNS: &'static [&'static str] = &[
    "Download",
    "Title",
    "Authors",
    "Series",
    "Year",
    "Language",
    "Publisher",
    "Duplicates",
    "FileSize",
    "Format",
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
            uifilter,
            download,
            download_status,
        } = self;

        if let Some(db) = db {
            for _ in 0..10000 {
                match db.get_result() {
                    Some(Ok(newbooks)) => {
                        read_results(results, newbooks, uifilter, filters.deduplicate, ctx)
                    }
                    Some(Err(e)) => *results = Err(e.to_string()),
                    None => break,
                }
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
            let f = filters.collection == Fiction;
            if ui.selectable_label(f, "Fiction").clicked() {
                filters.collection = Fiction
            } else if ui.selectable_label(!f, "Nonfiction").clicked() {
                filters.collection = NonFiction
            } else {
                changed = false;
            }

            ui.checkbox(&mut filters.deduplicate, "Remove duplicates");

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
                    *uifilter = UIFilter::default();
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

            ui.separator();
            if let Some(status) = download.get_status() {
                *download_status = status;
            }
            ui.label(format!("Downloaded: {:?}", download_status));
        });

        egui::CentralPanel::default().show(ctx, |ui| match results {
            Ok(books) => render_results_table(ui, books, download),
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
    uifilter: &mut UIFilter,
    deduplicate: bool,
    ctx: &egui::Context,
) {
    if let Ok(bookcache) = results {
        while newbooks.len() > 0 {
            let mut newbook = newbooks.pop().unwrap();
            if deduplicate {
                filter_update_booklist(uifilter, bookcache, &mut newbook);
            } else {
                bookcache.push(newbook);
            }
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

fn render_results_table(
    ui: &mut egui::Ui,
    books: &mut Vec<db::Book>,
    download: &download::Download,
) {
    let mut tb = TableBuilder::new(ui);
    for col in COLUMNS.iter() {
        let minwidth = match *col {
            "Title" => 200.0,
            "Authors" | "Series" | "Publisher" => 100.0,
            _ => 80.0,
        };
        tb = tb.column(
            Column::auto()
                .range(RangeInclusive::new(minwidth, 3000.0))
                .resizable(true)
                .clip(true),
        );
    }
    tb.header(20.0, |mut header| {
        for col in COLUMNS.iter() {
            header.col(|ui| {
                if ui.button(*col).clicked() {
                    sort_books(col, books);
                }
            });
        }
    })
    .body(|body| {
        body.rows(20.0, books.len(), |i, mut row| {
            row.col(|ui| {
                if ui.button("download").clicked() {
                    if let Err(_) = download.queue.send(books[i].clone()) {
                        eprintln!("Failed to send download request");
                    }
                }
            });
            render_text_cell(&mut row, books[i].title.as_str());
            render_text_cell(&mut row, books[i].authors.as_str());
            render_text_cell(&mut row, books[i].series.as_str());
            render_text_cell(&mut row, books[i].year.as_str());
            render_text_cell(&mut row, books[i].language.as_str());
            render_text_cell(&mut row, books[i].publisher.as_str());
            render_text_cell(&mut row, books[i].duplicates.to_string().as_str());
            render_text_cell(
                &mut row,
                format!("{:.0}", books[i].sizeinbytes as f32 / 1024.0).as_str(),
            );
            render_text_cell(&mut row, books[i].format.as_str());
        });
    });
}

fn sort_books(col: &&str, books: &mut Vec<db::Book>) {
    books.sort_by(|a, b| match *col {
        "Title" => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
        "Authors" => a.authors.to_lowercase().cmp(&b.authors.to_lowercase()),
        "Series" => compare_series(a.series.as_str(), b.series.as_str()),
        "Year" => a.year.cmp(&b.year),
        "Language" => a.language.cmp(&b.language),
        "Publisher" => a.publisher.cmp(&b.publisher),
        "Duplicates" => b.duplicates.cmp(&a.duplicates),
        "FileSize" => a.sizeinbytes.cmp(&b.sizeinbytes),
        "Format" => a.format.to_lowercase().cmp(&b.format.to_lowercase()),
        &_ => Ordering::Equal,
    });
}

fn render_text_cell(row: &mut egui_extras::TableRow<'_, '_>, text: &str) {
    row.col(|ui| {
        ui.label(text);
    });
}

// parse out the number at the end of the series name
fn compare_series(a: &str, b: &str) -> Ordering {
    let a = a.trim();
    let b = b.trim();
    let a = a.split_whitespace().last();
    let b = b.split_whitespace().last();
    match (a, b) {
        (Some(a), Some(b)) => match (a.parse::<i32>(), b.parse::<i32>()) {
            (Ok(a), Ok(b)) => a.cmp(&b),
            _ => a.cmp(b),
        },
        _ => a.cmp(&b),
    }
}
