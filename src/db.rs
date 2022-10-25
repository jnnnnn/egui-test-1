use std::{
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
};

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

#[derive(Debug, Clone)]
pub struct Params {
    pub collection: String,
    pub title: String,
    pub authors: String,
    pub series: String,
    pub language: String,
    pub format: String,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            collection: "fiction".to_owned(),
            title: "".to_owned(),
            authors: "".to_owned(),
            series: "".to_owned(),
            language: "".to_owned(),
            format: "".to_owned(),
        }
    }
}

impl DB {
    pub fn new(conn: &str) -> Self {
        let (query_send, query_receive) = mpsc::channel::<Query>();
        let (response_send, response_receive) = mpsc::channel::<Result<Vec<Book>, String>>();
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
            let query = query_receive.recv().unwrap();
            processing_clone.store(true, Relaxed);
            println!("query: {:?}", query);
            let stmt = connection.prepare(&query.stmt);
            if let Err(e) = stmt {
                eprintln!("Error preparing statement: {}", e);
                response_send.send(Err(e.to_string())).unwrap();
                continue;
            }
            let mut stmt = stmt.unwrap();
            let rows = stmt.query(rusqlite::named_params!(
                ":title": query.params.title,
                ":authors": query.params.authors,
                ":series": query.params.series,
                ":language": query.params.language,
                ":format": query.params.format,
            ));
            if let Err(e) = rows {
                eprintln!("Error executing statement: {}", e);
                response_send.send(Err(e.to_string())).unwrap();
                continue;
            }
            let mut rows = rows.unwrap().mapped(|row| row_to_book(row));
            loop {
                match rows.next() {
                    Some(Ok(book)) => {
                        println!("Sending a book: {:}", book.title);
                        response_send.send(Ok(vec![book])).unwrap();
                    }
                    Some(Err(e)) => response_send
                        .send(Err(format!("DB error: {:?}", e)))
                        .unwrap(),
                    None => break,
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
        ", match params.collection.as_str() {
            "nonfiction" => "non_fiction",
            _ => "fiction",
        });
        self.query_send.send(Query { stmt, params }).unwrap();
    }

    pub fn get_result(&self) -> Option<Result<Vec<Book>, String>> {
        match self.response_receive.try_recv() {
            Ok(Ok(books)) => Some(Ok(books)),
            Ok(Err(err)) => Some(Err(err)),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => None,
        }
    }

    pub fn interrupt(&self) {
        if let Some(interrupt) = &self.interrupt {
            println!("Interrupting query");
            interrupt.interrupt();
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
