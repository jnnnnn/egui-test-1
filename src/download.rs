use config::Config;
use crossbeam::channel::{unbounded, Receiver, Sender};
use std::{collections::HashMap, error, thread};

#[derive(Debug, Default, Clone)]
pub struct Status {
    pub completed: u64,
    pub errors: u64,
    pub description: String,
}

pub struct Download {
    pub settings: HashMap<String, String>,
    pub queue: Sender<String>,
    pub status: Receiver<Status>,
}

impl Download {
    pub fn new() -> Self {
        let (queue, recv) = unbounded::<String>();
        let (status_send, status_recv) = unbounded::<Status>();
        let settings = load_settings();
        let mut status = Status::default();

        // run downloads off the UI thread
        thread::spawn(move || loop {
            if let Ok(locator) = recv.recv() {
                if let Err(e) = start_download(&locator, &mut status, &status_send) {
                    status.description = format!("Error: {}", e);
                    status.errors += 1;
                    if let Err(e) = status_send.send(status.clone()) {
                        eprintln!("Error sending error: {}", e);
                    }
                }
            }
        });

        Self {
            settings,
            queue,
            status: status_recv,
        }
    }

    pub fn get_status(&self) -> Option<Status> {
        self.status.try_recv().ok()
    }
}

fn start_download(
    locator: &str,
    status: &mut Status,
    status_send: &Sender<Status>,
) -> Result<(), Box<dyn error::Error>> {
    status.description = format!("Downloading {}", locator);
    status.completed += 1;
    status_send.send(status.clone())?;
    Ok(())
}

fn load_settings() -> HashMap<String, String> {
    Config::builder()
        // Add in `./Settings.*`
        .add_source(config::File::with_name("Settings"))
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        .add_source(config::Environment::with_prefix("APP"))
        .build()
        .unwrap_or_default()
        .try_deserialize::<HashMap<String, String>>()
        .unwrap_or_default()
}
