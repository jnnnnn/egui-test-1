use bytes::Bytes;
use config::Config;
use crossbeam::channel::{unbounded, Receiver, Sender};
use fstrings::{f, format_args_f, println_f};
use std::{
    error,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};
use tokio::task::JoinSet;

use crate::db::Book;

#[derive(Debug, Default, Clone)]
pub struct Status {
    pub completed: u64,
    pub errors: u64,
    pub description: String,
}

pub struct Download {
    pub queue: Sender<Book>,
    pub status: Receiver<Status>,
}

impl Download {
    pub fn new() -> Self {
        let (queue, recv) = unbounded::<Book>();
        let (status_send, status_recv) = unbounded::<Status>();
        let config = load_settings();
        let mut status = Status::default();

        // run downloads off the UI thread
        thread::spawn(move || loop {
            if let Ok(book) = recv.recv() {
                if let Err(e) = start_download(&book, &mut status, &status_send, &config) {
                    eprintln!("Error downloading book {}: {}", book.title, e);
                    status.description = format!("Error: {}", e);
                    status.errors += 1;
                    if let Err(e) = status_send.send(status.clone()) {
                        eprintln!("Error sending error: {}", e);
                    }
                }
            } else {
                break;
            }
        });

        Self {
            queue,
            status: status_recv,
        }
    }

    pub fn get_status(&self) -> Option<Status> {
        self.status.try_recv().ok()
    }
}

fn start_download(
    book: &Book,
    status: &mut Status,
    status_send: &Sender<Status>,
    config: &Config,
) -> Result<(), Box<dyn error::Error>> {
    status.description = format!("Downloading {}", book.title);

    let author_subfolder = config.get::<bool>("authorSubfolder").unwrap_or_default();
    let filename = match author_subfolder {
        true => PathBuf::from(&book.authors)
            .join(&book.title)
            .with_extension(&book.format),
        false => PathBuf::from(format!("{} - {}", &book.authors, &book.title))
            .with_extension(&book.format),
    };
    let download_path = config.get::<String>("downloadPath")?;
    let path = &Path::new(&download_path).join(filename);

    if path.exists() {
        status.description = f!("{book.title} already exists");
        status_send.send(status.clone())?;
        println!("{} already exists at {}", book.title, path.display());
        return Ok(());
    }

    let hosts: Vec<String> = config
        .get::<String>("url_ipfs_hosts")?
        .split_ascii_whitespace()
        .map(|s| s.to_string())
        .collect();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let maybe_bytes = runtime.block_on(download_race(hosts, book));
    match maybe_bytes {
        Ok(bytes) => {
            status.description = f!("Writing {book.title} to disk");
            status_send.send(status.clone())?;
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(path, bytes)?;
            println!("Wrote {}", path.display());
            status.completed += 1;
            status.description = f!("Downloaded {book.title}");
            status_send.send(status.clone())?;
        }
        Err(e) => {
            status.description = format!("Error downloading {}: {}", book.title, e);
            status.errors += 1;
            status_send.send(status.clone())?;
        }
    }
    Ok(())
}

async fn download_race(hosts: Vec<String>, book: &Book) -> Result<Bytes, String> {
    // start a download for each host
    let mut set = JoinSet::<Result<Bytes, String>>::new();

    for (i, host) in hosts.iter().enumerate() {
        let host = host.clone();
        let book = book.clone();
        set.spawn(async move {
            // give each endpoint an extra ten seconds to start
            let delay = Duration::from_secs(10 * i as u64);
            tokio::time::sleep(delay).await;
            download_file(&host, &book).await.map_err(|e| e.to_string())
        });
    }
    while let Some(result) = set.join_next().await {
        match result {
            Ok(Ok(bytes)) => {
                set.abort_all(); // abort the rest of the downloads
                return Ok(bytes);
            }
            Ok(Err(e)) => eprintln!("Error downloading: {}", e),
            Err(e) => eprintln!("Error joining download: {}", e),
        }
    }
    Err("No downloads succeeded".to_string())
}

async fn download_file(host: &String, book: &Book) -> Result<Bytes, Box<dyn error::Error>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let series = if book.series.is_empty() {
        f!("")
    } else {
        f!("({book.series}) ")
    };
    let filename = f!("{series}{book.authors} - {book.title}.{book.format}");
    let url = f!("https://{host}/ipfs/{book.ipfs_cid}?filename={filename}");

    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        return Err(format!("Error downloading {}: {}", url, response.status()).into());
    }
    let result = response.bytes().await.map_err(|e| e.into());
    if result.is_ok() {
        println_f!("Downloaded {}", url);
    }
    result
}

fn load_settings() -> Config {
    let config = Config::builder()
        // Add in `./Settings.*`
        .add_source(config::File::with_name("Settings"))
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        .add_source(config::Environment::with_prefix("APP"))
        .build();

    if let Err(e) = &config {
        eprintln!("Error loading config: {}", e);
    }

    config.unwrap_or_default()
}
