use std::{
    error,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc,
    },
    thread,
};

use crossbeam::channel::{unbounded, Receiver, Sender};
use rusqlite::{InterruptHandle, Row};

pub struct DB {
    query_send: Sender<Query>,
    response_receive: Receiver<Result<Vec<Book>, String>>,
    interrupt: Option<InterruptHandle>,
    pub processing: Arc<AtomicBool>,
}

#[derive(Debug)]
pub struct Book {
    pub title: String,
    pub authors: String,
    pub series: String,
    pub year: String,
    pub language: String,
    pub publisher: String,
    pub sizeinbytes: i64,
    pub format: String,
    pub locator: String,
}

#[derive(Debug)]
pub struct Query {
    pub stmt: String,
    pub params: Params,
}

#[derive(Debug, Default, Clone)]
pub struct Params {
    pub collection: Collection,
    pub title: String,
    pub authors: String,
    pub series: String,
    pub language: String,
    pub format: String,
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub enum Collection {
    #[default]
    Fiction,
    NonFiction,
}

impl DB {
    // open a new DB connection.
    pub fn new(conn: &str) -> Self {
        let (query_send, query_receive) = unbounded::<Query>();
        let (response_send, response_receive) = unbounded::<Result<Vec<Book>, String>>();
        let conn = conn.to_owned();

        let processing = Arc::new(AtomicBool::new(false));

        let connection = rusqlite::Connection::open(&conn);
        if let Err(e) = connection {
            eprintln!("Error opening database: {}", e);
            return Self {
                query_send,
                response_receive,
                interrupt: None,
                processing,
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
        }
    }

    // add a query to the queue for the db thread to process
    pub fn query(&self, params: Params) {
        // https://www.sqlite.org/fts5.html
        // search the fiction_fts table for query
        let stmt = format!("
        SELECT f.title, f.authors, f.series, f.year, f.language, f.publisher, f.sizeinbytes, f.format, f.locator 
        FROM {} f
        WHERE 
            f.title LIKE '%'||:title||'%' AND 
            f.authors LIKE '%'||:authors||'%' AND
            f.series LIKE '%'||:series||'%' AND
            f.language LIKE '%'||:language||'%' AND
            f.format LIKE '%'||:format||'%'
        ORDER BY f.authors, f.title, f.sizeinbytes
        ", match params.collection {
            Collection::NonFiction => "non_fiction",
            _ => "fiction",
        });
        if let Err(e) = self.query_send.send(Query { stmt, params }) {
            eprintln!("Error enqueueing query: {}", e);
        }
    }

    // see if there's a result available from the db thread
    pub fn get_result(&self) -> Option<Result<Vec<Book>, String>> {
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
    response_send: &Sender<Result<Vec<Book>, String>>,
) -> Result<(), Box<dyn error::Error>> {
    let mut stmt = connection.prepare(&query.stmt)?;
    let rows = stmt.query(rusqlite::named_params!(
        ":title": query.params.title,
        ":authors": query.params.authors,
        ":series": query.params.series,
        ":language": query.params.language,
        ":format": query.params.format,
    ));
    let mut rows = rows?.mapped(|row| row_to_book(row));
    loop {
        match rows.next() {
            Some(Ok(book)) => response_send.send(Ok(vec![book]))?,
            Some(Err(e)) => return Err(e.into()),
            None => break Ok(()),
        }
    }
}

fn row_to_book(row: &Row<'_>) -> Result<Book, rusqlite::Error> {
    Ok(Book {
        title: row.get(0)?,
        authors: row.get(1)?,
        series: row.get(2)?,
        year: row.get(3)?,
        language: row.get(4)?,
        publisher: row.get(5)?,
        sizeinbytes: row.get(6)?,
        format: row.get(7)?,
        locator: row.get(8)?,
    })
}
