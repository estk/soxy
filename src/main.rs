mod config;
mod error;

use crate::config::Config;
use actix_web::client::Client;
use actix_web::error as actix_error;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use error::{Error, Result};
use futures::{Future, IntoFuture};
use http::header;
use std::net::SocketAddr;
use url::Url;


const OUT_IP: [u8; 4] = [127, 0, 0, 1];
const CONFIG_PATH: &str = "config.toml";

fn main() -> Result<()> {
    let config = config::parse_config(CONFIG_PATH)?;
    let listen_addr = config.listen_addr;

    HttpServer::new(move || {
        App::new()
            .data(Client::new())
            .data(config.clone())
            .wrap(middleware::Logger::default())
            .default_service(web::route().to_async(forward))
    })
    .bind(listen_addr)
    .map_err(|source| Error::BindPort {
        listen_addr,
        source,
    })?
    .system_exit()
    .run()
    .map_err(|source| Error::Run { source })
}

fn forward(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
    config: web::Data<Config>,
) -> impl Future<Item = HttpResponse, Error = actix_error::Error> {
    map_request(req.clone(), &config)
        .map_err(actix_error::ErrorNotFound)
        .into_future()
        .and_then(move |new_url| {
            client
                .request_from(new_url.as_str(), req.head())
                .if_some(req.head().peer_addr, |addr, fr| {
                    fr.header("x-forwarded-for", format!("{}", addr.ip()))
                })
                .send_stream(payload)
                .map(|res| {
                    let mut client_resp = HttpResponse::build(res.status());
                    for (header_name, header_value) in
                        res.headers().iter().filter(|(h, _)| *h != "connection")
                    {
                        client_resp.header(header_name.clone(), header_value.clone());
                    }
                    client_resp.streaming(res)
                })
                .map_err(actix_error::Error::from)
        })
}

fn map_request(req: HttpRequest, config: &Config) -> Result<Url> {
    let socket_addr = req
        .headers()
        .get(header::HOST)
        .ok_or(Error::HostEmpty)
        .and_then(|host| get_addr(host, config))?;

    let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");

    let url = format!("http://{}{}", socket_addr, path);
    url.parse::<Url>()
        .map_err(|source| Error::InvalidUpstreamUrl { url, source })
}

fn get_addr(host_value: &header::HeaderValue, config: &Config) -> Result<SocketAddr> {
    let host = host_value
        .to_str()
        .map_err(|source| Error::HostReadError { source })?
        .to_owned();

    let out_port = config
        .domains
        .get(&host)
        .ok_or_else(|| Error::HostNotFound { host })?
        .to_owned();

    Ok((OUT_IP, out_port).into())
}