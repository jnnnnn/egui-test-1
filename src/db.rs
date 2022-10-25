use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use rusqlite::{InterruptHandle, Row};

pub struct DB {
    pub result: Option<Vec<Book>>,
    query_send: Sender<Query>,
    response_receive: Receiver<Result<Vec<Book>, String>>,
    interrupt: Option<InterruptHandle>,
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
    pub title: String,
    pub authors: String,
    pub series: String,
    pub language: String,
    pub format: String,
}

impl DB {
    pub fn new(conn: &str) -> Self {
        let (query_send, query_receive) = mpsc::channel::<Query>();
        let (response_send, response_receive) = mpsc::channel::<Result<Vec<Book>, String>>();
        let conn = conn.to_owned();

        // print SQLite version
        println!("SQLite version: {}", rusqlite::version());

        let connection = rusqlite::Connection::open(&conn);
        if let Err(e) = connection {
            eprintln!("Error opening database: {}", e);
            return Self {
                result: None,
                query_send,
                response_receive,
                interrupt: None,
            };
        }
        let connection = connection.unwrap();
        let interrupt = Some(connection.get_interrupt_handle());
        // queries run in a separate thread
        // https://doc.rust-lang.org/rust-by-example/std_misc/channels.html
        thread::spawn(move || loop {
            let query = query_receive.recv().unwrap();
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
            result: None,
            query_send,
            response_receive,
            interrupt,
        }
    }

    pub fn query(&self, params: Params) {
        // https://www.sqlite.org/fts5.html
        // search the fiction_fts table for query
        let stmt = String::from("
        SELECT f.title, f.authors, f.series, f.year, f.language, f.publisher, f.sizeinbytes, f.format, f.locator 
        FROM fiction f
        WHERE 
            f.title LIKE '%'||:title||'%' AND 
            f.authors LIKE '%'||:authors||'%' AND
            f.series LIKE '%'||:series||'%' AND
            f.language LIKE '%'||:language||'%' AND
            f.format LIKE '%'||:format||'%'
        ORDER BY f.authors, f.title, f.sizeinbytes
        ");
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
