use config::Config;
use std::collections::HashMap;

pub struct Download {
    pub settings: HashMap<String, String>,
}

impl Download {
    pub fn new() -> Self {
        Self {
            settings: Config::builder()
                // Add in `./Settings.*`
                .add_source(config::File::with_name("Settings"))
                // Add in settings from the environment (with a prefix of APP)
                // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
                .add_source(config::Environment::with_prefix("APP"))
                .build()
                .unwrap_or_default()
                .try_deserialize::<HashMap<String, String>>()
                .unwrap_or_default(),
        }
    }
}
