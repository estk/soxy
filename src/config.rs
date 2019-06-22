use std::net::SocketAddr;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use toml::Value;
use serde_derive::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub listen_addr: SocketAddr,
    pub domains: HashMap<String, u16>
}
/// Read the `config.toml` at `path`.
pub fn parse_config(path: &str) -> Result<Config, Box<dyn Error>> {
    let mut input = File::open(path)?;
    let mut config_content = String::new();
    input.read_to_string(&mut config_content)?;
    let config: Value = config_content.parse()?;

    config
        .to_owned()
        .try_into()
        .map_err(|e| e.into())
}
