use config::Config;

pub fn load_settings() -> Config {
    let config = Config::builder()
        // Add in `./Settings.*`
        .add_source(config::File::with_name("Settings"))
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        .add_source(config::Environment::with_prefix("APP"))
        .build();

    if let Err(e) = &config {
        log::error!("Error loading config: {}", e);
    }

    config.unwrap_or_default()
}
