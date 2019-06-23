use crate::error::{Error, Result};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use toml::Value;


#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Config {
    pub listen_addr: SocketAddr,
    pub domains: HashMap<String, u16>,
}

/// Read the `config.toml` at `path`.
pub fn parse_config(path: &str) -> Result<Config> {
    let mut input = File::open(path).map_err(|source| Error::OpenConfig {
        path: path.to_owned(),
        source,
    })?;
    let mut buf = String::new();
    input
        .read_to_string(&mut buf)
        .map_err(|source| Error::ConfigRead { source })?;
    let config: Value = buf
        .parse()
        .map_err(|source| Error::ConfigParse { source })?;

    config
        .to_owned()
        .try_into()
        .map_err(|source| Error::ConfigSchema { source })
}
