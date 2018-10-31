use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use toml::Value;

/// Read the `config.toml` at `path`.
pub fn read_config(path: &str) -> Result<HashMap<String, u16>, Box<dyn Error>> {
    let mut input = File::open(path)?;
    let mut config_content = String::new();
    input.read_to_string(&mut config_content)?;
    let config: Value = config_content.parse()?;

    config["domains"]
        .to_owned()
        .try_into()
        .map_err(|e| e.into())
}
