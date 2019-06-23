use http;
use snafu::Snafu;
use std::io;
use std::net::SocketAddr;


pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not open config from {}: {}", path, source))]
    OpenConfig {
        path: String,
        source: std::io::Error,
    },

    #[snafu(display("Could not read config file: {}", source))]
    ConfigRead { source: io::Error },

    #[snafu(display("Could not parse config file: {}", source))]
    ConfigParse { source: toml::de::Error },

    #[snafu(display("Config file schema does not match struct: {}", source))]
    ConfigSchema { source: toml::de::Error },

    #[snafu(display("Could not bind to port {}: {}", listen_addr, source))]
    BindPort {
        listen_addr: SocketAddr,
        source: io::Error,
    },

    #[snafu(display("Could not run the server: {}", source))]
    Run { source: io::Error },

    #[snafu(display("Host header was empty"))]
    HostEmpty,

    #[snafu(display("Host header was invalid"))]
    HostReadError { source: http::header::ToStrError },

    #[snafu(display("Host not found in config {}", host))]
    HostNotFound { host: String },

    #[snafu(display(
        "The configuration and host provided resulted in an invalid url {}: {}",
        url,
        source
    ))]
    InvalidUpstreamUrl {
        url: String,
        source: url::ParseError,
    },
}