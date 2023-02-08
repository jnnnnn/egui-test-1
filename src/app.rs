use std::{
    cmp::Ordering,
    ops::RangeInclusive,
    path::PathBuf,
    sync::{atomic::Ordering::Relaxed, RwLock},
};

use config::Config;
use egui_extras::{Column, TableBuilder};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::{
    config::load_settings,
    db::{
        self, BookRef,
        Collection::{Fiction, NonFiction},
    },
    download,
    uifilter::{filter_update_booklist, UIFilter},
};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
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
    results: Result<Vec<db::BookRef>, String>,
    #[serde(skip)]
    uifilter: UIFilter,
    #[serde(skip)]
    config: Config,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            value: 2.7,
            filters: db::Params::default(),
            db: None,
            results: Err(String::from("No results")),
            download: download::Download::new(),
            download_status: download::Status::default(),
            uifilter: UIFilter::default(),
            config: load_settings(),
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
        log::info!("App started");
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
            value: _,
            filters,
            db,
            results,
            uifilter,
            download,
            download_status,
            config,
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
        } else {
            *db = Some(db::DB::new());
        }

        // For inspiration and more examples, go to https://emilk.github.io/egui
        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Side Panel");

            let mut changed = true;
            let f = filters.collection == Fiction;
            if ui.selectable_label(f, "Fiction").clicked() {
                filters.collection = Fiction
            } else if ui.selectable_label(!f, "Nonfiction").clicked() {
                filters.collection = NonFiction
            } else {
                changed = false;
            }

            changed |= ui
                .checkbox(&mut filters.deduplicate, "Remove duplicates")
                .changed();

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

        egui::CentralPanel::default().show(ctx, |ui| {
            match results {
                Ok(books) => render_results_table(ui, books, download, config),
                Err(e) => {
                    ui.label(e.to_string());
                    ()
                }
            };
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            render_logs(ui);
        });
    }
}

fn render_logs(ui: &mut egui::Ui) -> () {
    ui.separator();
    let logtext = "".to_string();
    ui.add(egui::TextEdit::multiline(&mut logtext.as_str()));
}

fn read_results(
    results: &mut Result<Vec<db::BookRef>, String>,
    mut newbooks: Vec<db::BookRef>,
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
    books: &mut Vec<db::BookRef>,
    download: &download::Download,
    config: &config::Config,
) {
    let link_base = config.get::<String>("linkBase").unwrap_or("".to_string());
    let mut tb = TableBuilder::new(ui)
        .max_scroll_height(10_000.0)
        .striped(true);
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
            render_download_cell(&mut row, download, &books[i]);
            let authors = books[i].authors.as_str();
            let title_query = format!("{} by {}", books[i].title, authors);
            render_searchlink_cell(&mut row, books[i].title.as_str(), &link_base, &title_query);
            render_searchlink_cell(&mut row, authors, &link_base, authors);
            render_text_cell(&mut row, books[i].series.as_str());
            render_text_cell(&mut row, books[i].year.as_str());
            render_text_cell(&mut row, books[i].language.as_str());
            render_text_cell(&mut row, books[i].publisher.as_str());
            let dups = books[i].duplicates.read().unwrap().to_string();
            render_text_cell(&mut row, &dups);
            render_text_cell(
                &mut row,
                format!("{:.0}", books[i].sizeinbytes as f32 / 1024.0).as_str(),
            );
            render_text_cell(&mut row, books[i].format.as_str());
        });
    });
}

fn render_download_cell(
    row: &mut egui_extras::TableRow<'_, '_>,
    download: &download::Download,
    book: &db::BookRef,
) {
    let mut download_status = if let Ok(status) = book.download_status.read() {
        status.clone()
    } else {
        String::from("")
    };
    if download_status == "?" {
        download_status = check_downloaded(book);
    }

    row.col(|ui| match download_status {
        s if s == "" => {
            if ui.button("download").clicked() {
                if let Ok(mut status) = book.download_status.write() {
                    *status = String::from("Queued");
                }
                if let Err(_) = download.queue.send(book.clone()) {
                    log::error!("Failed to send download request");
                }
            }
        }
        s if s == "Done" => {
            if ui.button("open").clicked() {
                if let Err(e) =
                    open::that(book.download_path.parent().unwrap_or(&PathBuf::from(".")))
                {
                    log::error!("Failed to open file: {}", e);
                }
            }
        }
        s => {
            ui.label(s);
        }
    });
}

fn check_downloaded(book: &BookRef) -> String {
    let mut status = String::from("");
    if let Ok(path) = book.download_path.canonicalize() {
        if path.exists() {
            status = String::from("Done");
        }
    }
    if let Ok(mut status_ref) = book.download_status.write() {
        *status_ref = status.clone();
    }
    return status;
}

fn sort_books(col: &&str, books: &mut Vec<db::BookRef>) {
    books.sort_by(|a, b| match *col {
        "Title" => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
        "Authors" => a.authors.to_lowercase().cmp(&b.authors.to_lowercase()),
        "Series" => compare_series(a.series.as_str(), b.series.as_str()),
        "Year" => a.year.cmp(&b.year),
        "Language" => a.language.cmp(&b.language),
        "Publisher" => a.publisher.cmp(&b.publisher),
        "Duplicates" => compare_duplicates(&b.duplicates, &a.duplicates),
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

fn render_searchlink_cell(
    row: &mut egui_extras::TableRow<'_, '_>,
    text: &str,
    link_base: &str,
    query: &str,
) {
    let query = utf8_percent_encode(query, NON_ALPHANUMERIC).to_string();
    if link_base == "" {
        render_text_cell(row, text);
    } else {
        let url = format!("{}{}", link_base, query);
        row.col(|ui| {
            ui.hyperlink_to(text, url);
        });
    }
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

fn compare_duplicates(a: &RwLock<usize>, b: &RwLock<usize>) -> Ordering {
    let a = a.read().unwrap();
    let b = b.read().unwrap();
    a.cmp(&b)
}
