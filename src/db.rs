use std::{
    error,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, Mutex,
    },
    thread,
};

use crossbeam::{
    channel::{unbounded, Receiver, Sender},
    select,
};
use sqlx::{Connection, FromRow, Row, SqliteConnection};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;

struct Interrupt {}

pub struct DB {
    query_send: Sender<Query>,
    response_receive: Receiver<Result<Vec<Book>, String>>,
    interrupt_send: Sender<Interrupt>,
    pub processing: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
pub struct Book {
    pub title: String,
    pub authors: String,
    pub series: String,
    pub year: String,
    pub language: String,
    pub publisher: String,
    pub sizeinbytes: i64,
    pub format: String,
    pub hash: String,
    pub collection: Collection,
}

// implement the trait bound `Book: From<SqliteRow>`.
// This is required because sqlx::FromRow is not implemented for enums.
//
// https://docs.rs/sqlx/0.5.9/sqlx/trait.FromRow.html
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for Book {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            title: row.try_get("title")?,
            authors: row.try_get("authors")?,
            series: row.try_get("series")?,
            year: row.try_get("year")?,
            language: row.try_get("language")?,
            publisher: row.try_get("publisher")?,
            sizeinbytes: row.try_get("sizeinbytes")?,
            format: row.try_get("format")?,
            hash: row.try_get("hash")?,
            // map from string to enum
            collection: match row.try_get::<String, _>("collection")?.as_str() {
                "Fiction" => Collection::Fiction,
                "NonFiction" => Collection::NonFiction,
                _ => Collection::Fiction,
            },
        })
    }
}

#[derive(Debug)]
pub struct Query {
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
}

#[derive(Debug, Default, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
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
        let (interrupt_send, interrupt_receive) = unbounded::<Interrupt>();
        let conn = conn.to_owned();

        let processing = Arc::new(AtomicBool::new(false));
        let processing_clone = processing.clone();

        let interrupt = Arc::new(AtomicBool::new(false));
        let interrupt_clone = interrupt.clone();
        // queries run in a separate thread
        // https://doc.rust-lang.org/rust-by-example/std_misc/channels.html
        thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(start_connection(
                conn,
                query_receive,
                response_send,
                interrupt_receive,
                processing_clone,
            ));
        });

        Self {
            query_send,
            response_receive,
            interrupt_send,
            processing,
        }
    }

    pub fn query(&self, filters: Params) {
        let query = Query { params: filters };
        if let Err(e) = self.query_send.send(query) {
            eprintln!("Error sending query: {}", e);
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
        if let Err(e) = self.interrupt_send.send(Interrupt {}) {
            eprintln!("Error sending interrupt: {}", e);
        }
    }
}

async fn start_connection(
    conn: String,
    query_receive: Receiver<Query>,
    response_send: Sender<Result<Vec<Book>, String>>,
    interrupt_receive: Receiver<Interrupt>,
    processing: Arc<AtomicBool>,
) {
    // a mutable connection
    let connection = SqliteConnection::connect(&conn).await;
    match connection {
        Ok(mut connection) => loop {
            listen(
                &mut connection,
                &query_receive,
                &response_send,
                &interrupt_receive,
                &processing,
            );
        },
        Err(e) => {
            eprintln!("Error opening database: {}", e);
        }
    }
}

async fn listen(
    connection: &mut SqliteConnection,
    query_receive: &Receiver<Query>,
    response_send: &Sender<Result<Vec<Book>, String>>,
    interrupt_receive: &Receiver<Interrupt>,
    processing: &Arc<AtomicBool>,
) {
    // a handle to the current query
    let join_handle: Arc<Mutex<Option<JoinHandle<()>>>> = Arc::new(Mutex::new(None));
    // wait for a query or an interrupt
    crossbeam::select! {
        recv(query_receive) -> query => {
            match query {
                Ok(query) => {
                    // start a new query
                    let task_handle = Some(tokio::spawn(async move {
                        start_query(connection, &query, &response_send, &processing).await;
                    }));
                    // store the handle so we can interrupt it
                    *join_handle.lock().unwrap() = task_handle;
                },
                Err(e) => {
                    eprintln!("Error receiving query: {}", e);
                }
            }
        },
        recv(interrupt_receive) -> interrupt => {
            match interrupt {
                Ok(_) => {
                    // interrupt the current query
                    processing.store(false, Relaxed);
                    // wait for the query to finish
                    if let Some(join_handle) = join_handle.lock().unwrap().take() {
                        join_handle.await.unwrap();
                    }
                },
                Err(e) => {
                    eprintln!("Error receiving interrupt: {}", e);
                }
            }
        }
    }
}

async fn start_query(
    connection: &mut SqliteConnection,
    query: &Query,
    response_send: &Sender<Result<Vec<Book>, String>>,
    processing: &Arc<AtomicBool>,
) -> Result<(), Box<dyn error::Error>> {
    processing.store(true, Relaxed);
    let collection = match query.params.collection {
        Collection::Fiction => "Fiction",
        Collection::NonFiction => "NonFiction",
    };
    let prep = sqlx::query(
        "SELECT title, authors, series, year, language, publisher, sizeinbytes, format, hash
        FROM Books
        WHERE collection = ?1
        AND title LIKE ?2
        AND authors LIKE ?3
        AND series LIKE ?4
        AND language LIKE ?5
        AND format LIKE ?6
        ORDER BY title",
    )
    .bind(&collection)
    .bind(&query.params.title)
    .bind(&query.params.authors)
    .bind(&query.params.series)
    .bind(&query.params.language)
    .bind(&query.params.format);

    // read 100 rows at a time and send back to the UI, unless interrupted
    let mut rows = prep.fetch(connection);
    let mut books = Vec::new();
    while let Some(row) = rows.try_next().await? {
        let book = Book::from_row(&row)?;
        books.push(book);
        if books.len() == 100 {
            if let Err(e) = response_send.send(Ok(books)) {
                eprintln!("Error sending books: {}", e);
            }
            books = Vec::new();
        }
    }
    if !books.is_empty() {
        if let Err(e) = response_send.send(Ok(books)) {
            eprintln!("Error sending books: {}", e);
        }
    }

    processing.store(false, Relaxed);

    Ok(())
}
