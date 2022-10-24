use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

pub struct DB {
    pub result: Option<Vec<Book>>,
    query_send: Sender<Query>,
    response_receive: Receiver<Result<Vec<Book>, String>>,
}

pub struct Book {
    pub title: String,
    pub authors: String,
    pub series: String,
    pub year: String,
    pub language: String,
    pub publisher: String,
    pub sizeinbytes: String,
    pub format: String,
    pub locator: String,
}

pub struct Query {
    pub stmt: String,
    pub params: Vec<String>,
}

impl DB {
    pub fn new(conn: &str) -> Self {
        let (query_send, query_receive) = mpsc::channel::<Query>();
        let (response_send, response_receive) = mpsc::channel::<Result<Vec<Book>, String>>();
        let conn = conn.to_owned();

        // print SQLite version
        println!("SQLite version: {}", rusqlite::version());

        // queries run in a separate thread
        // https://doc.rust-lang.org/rust-by-example/std_misc/channels.html
        thread::spawn(move || {
            let connection = rusqlite::Connection::open(&conn);
            if let Err(e) = connection {
                eprintln!("Error opening database: {}", e);
                return;
            }
            let connection = connection.unwrap();
            loop {
                let query = query_receive.recv().unwrap();
                let result = execute(&connection, &query);
                match result {
                    Ok(books) => response_send.send(Ok(books)).unwrap(),
                    Err(e) => response_send.send(Err(e.to_string())).unwrap(),
                }
            }
        });

        Self {
            result: None,
            query_send,
            response_receive,
        }
    }

    pub fn query(&self, query: &str) {
        // https://www.sqlite.org/fts5.html
        // search the fiction_fts table for query
        let stmt = String::from("
        SELECT f.title, f.authors, f.series, f.year, f.language, f.publisher, f.sizeinbytes, f.format, f.locator 
        FROM fiction_fts ft JOIN fiction f ON f.title = ft.title 
        WHERE fiction_fts MATCH '?'
        ");
        let params = vec![query.to_owned()];
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
}

fn execute(connection: &rusqlite::Connection, query: &Query) -> Result<Vec<Book>, rusqlite::Error> {
    let mut stmt = connection.prepare(&query.stmt)?;
    let rows = stmt.query(rusqlite::params_from_iter(query.params.iter()))?;
    rows.mapped(|row| {
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
    })
    .collect()
}
