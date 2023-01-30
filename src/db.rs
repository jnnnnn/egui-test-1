use std::{
    error,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, RwLock,
    },
    thread,
};

use config::Config;
use crossbeam::channel::{unbounded, Receiver, Sender};
use rusqlite::{InterruptHandle, Row};

use crate::config::load_settings;

pub struct DB {
    query_send: Sender<Query>,
    response_receive: Receiver<Result<Vec<BookRef>, String>>,
    interrupt: Option<InterruptHandle>,
    pub processing: Arc<AtomicBool>,
    config: Config,
}

#[derive(Debug, Default)]
pub struct Book {
    pub collection: Collection,
    pub title: String,
    pub authors: String,
    pub series: String,
    pub year: String,
    pub language: String,
    pub publisher: String,
    pub sizeinbytes: i64,
    pub format: String,
    pub ipfs_cid: String,
    pub duplicates: std::sync::RwLock<usize>,
    pub download_status: std::sync::RwLock<String>,
    pub download_path: PathBuf,
}

// a reference-counted public type for Book
pub type BookRef = Arc<Book>;

#[derive(Debug)]
pub struct Query {
    pub stmt: String,
    pub params: Params,
}

#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Params {
    pub collection: Collection,
    pub title: String,
    pub authors: String,
    pub series: String,
    pub language: String,
    pub format: String,
    pub deduplicate: bool,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Collection {
    #[default]
    Fiction,
    NonFiction,
}

impl DB {
    // open a new DB connection.
    pub fn new() -> Self {
        let (query_send, query_receive) = unbounded::<Query>();
        let (response_send, response_receive) = unbounded::<Result<Vec<BookRef>, String>>();
        
        let processing = Arc::new(AtomicBool::new(false));

        let config = load_settings();
        let conn = config.get::<String>("dbPath").unwrap_or("".to_string());

        // check that file exists
        if !std::path::Path::new(&conn).exists() {            
            eprintln!("Error opening database: {} does not exist", conn);
            return Self {
                query_send,
                response_receive,
                interrupt: None,
                processing,
                config,
            };
        }

        let connection = rusqlite::Connection::open(&conn);
        if let Err(e) = connection {
            eprintln!("Error opening database: {}", e);
            return Self {
                query_send,
                response_receive,
                interrupt: None,
                processing,
                config,
            };
        }
        let connection = connection.unwrap();
        let interrupt = Some(connection.get_interrupt_handle());

        let processing_clone = processing.clone();

        // queries run in a separate thread
        // https://doc.rust-lang.org/rust-by-example/std_misc/channels.html
        thread::spawn(move || loop {
            processing_clone.store(false, Relaxed);
            if let Ok(query) = query_receive.recv() {
                processing_clone.store(true, Relaxed);
                if let Err(e) = start_query(&connection, &query, &response_send) {
                    if let Err(e) = response_send.send(Err(e.to_string())) {
                        eprintln!("Error sending error: {}", e);
                    }
                }
            }
        });

        Self {
            query_send,
            response_receive,
            interrupt,
            processing,
            config,
        }
    }

    // add a query to the queue for the db thread to process
    pub fn query(&self, params: Params) {
        // https://www.sqlite.org/fts5.html
        // search the fiction_fts table for query
        let stmt = if self.config.get::<bool>("compressedDb").unwrap_or(false) {
            format!("
            SELECT f.title, f.author as authors, f.series, f.year, f.language, f.publisher, f.filesize as sizeinbytes, f.extension as format, f.ipfs_cid as ipfs_cid
            FROM {0} f
            WHERE 
                f.title LIKE '%'||:title||'%' AND 
                f.author LIKE '%'||:authors||'%' AND
                f.series LIKE '%'||:series||'%' AND
                f.language LIKE '%'||:language||'%' AND
                f.extension LIKE '%'||:format||'%'
            ORDER BY f.author, f.title, f.filesize
            ", match params.collection {
                Collection::NonFiction => "non_fiction",
                _ => "fiction",
            })
        }
        else {
            format!("
            SELECT f.title, f.author as authors, f.series, f.year, f.language, f.publisher, f.filesize as sizeinbytes, f.extension as format, fh.ipfs_cid as ipfs_cid
            FROM {0} f
            join {0}_hashes as fh on LOWER(f.md5) = fh.md5
            WHERE 
                f.title LIKE '%'||:title||'%' AND 
                f.author LIKE '%'||:authors||'%' AND
                f.series LIKE '%'||:series||'%' AND
                f.language LIKE '%'||:language||'%' AND
                f.extension LIKE '%'||:format||'%'
            ORDER BY f.author, f.title, f.filesize
            ", match params.collection {
                Collection::NonFiction => "non_fiction",
                _ => "fiction",
            })
        };
        if let Err(e) = self.query_send.send(Query { stmt, params }) {
            eprintln!("Error enqueueing query: {}", e);
        }
    }

    // see if there's a result available from the db thread
    pub fn get_result(&self) -> Option<Result<Vec<BookRef>, String>> {
        match self.response_receive.try_recv() {
            Ok(Ok(books)) => Some(Ok(books)),
            Ok(Err(err)) => Some(Err(err)),
            Err(_) => None,
        }
    }

    // stop the currently executing query.
    // It is also a good idea to drain the receive queue
    // (keep calling get_result until nothing is left).
    pub fn interrupt(&self) {
        if let Some(interrupt) = &self.interrupt {
            interrupt.interrupt();
        }
    }
}

fn start_query(
    connection: &rusqlite::Connection,
    query: &Query,
    response_send: &Sender<Result<Vec<BookRef>, String>>,
) -> Result<(), Box<dyn error::Error>> {
    let mut stmt = connection.prepare(&query.stmt)?;
    let rows = stmt.query(rusqlite::named_params!(
        ":title": query.params.title,
        ":authors": query.params.authors,
        ":series": query.params.series,
        ":language": query.params.language,
        ":format": query.params.format,
    ));
    let config = load_settings();
    let mut rows = rows?.mapped(|row| row_to_book(&config, query, row));
    loop {
        match rows.next() {
            Some(Ok(book)) => response_send.send(Ok(vec![book]))?,
            Some(Err(e)) => return Err(e.into()),
            None => break Ok(()),
        }
    }
}

fn row_to_book(config: &Config, query: &Query, row: &Row<'_>) -> Result<BookRef, rusqlite::Error> {
    let path = download_path(&config, &row.get(1)?, &row.get(0)?, &row.get(7)?);
    Ok(Arc::new(Book {
        collection: query.params.collection.clone(),
        title: row.get(0)?,
        authors: row.get(1)?,
        series: row.get(2)?,
        year: row.get(3)?,
        language: row.get(4)?,
        publisher: row.get(5)?,
        sizeinbytes: row.get(6)?,
        format: row.get(7)?,
        ipfs_cid: row.get(8)?,
        duplicates: RwLock::new(1),
        download_status: RwLock::new("?".to_string()),
        download_path: path,
    }))
}

fn download_path(config: &Config, authors: &String, title: &String, format: &String) -> PathBuf {
    let author_subfolder = config.get::<bool>("authorSubfolder").unwrap_or_default();
    let filename = match author_subfolder {
        true => PathBuf::from(authors).join(title).with_extension(format),
        false => PathBuf::from(format!("{} - {}", authors, title)).with_extension(format),
    };
    let download_path = config.get::<String>("downloadPath").unwrap_or_default();
    let path = &Path::new(&download_path).join(filename);
    return path.to_owned();
}
