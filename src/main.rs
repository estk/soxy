mod config;
mod error;

use futures::{Future, IntoFuture};
use crate::config::Config;

use actix_web::client::Client;
use actix_web::error as actix_error;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use error::{Error, Result};
use std::net::SocketAddr;
use url::Url;

const OUT_IP: [u8; 4] = [127, 0, 0, 1];

fn main() -> Result<()> {
    let config = config::parse_config("config.toml")?;
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
            let forwarded_req = client.request_from(new_url.as_str(), req.head());
            let forwarded_req = if let Some(addr) = req.head().peer_addr {
                forwarded_req.header("x-forwarded-for", format!("{}", addr.ip()))
            } else {
                forwarded_req
            };

            forwarded_req
                .send_stream(payload)
                .map_err(actix_error::Error::from)
                .map(|res| {
                    let mut client_resp = HttpResponse::build(res.status());
                    for (header_name, header_value) in
                        res.headers().iter().filter(|(h, _)| *h != "connection")
                    {
                        client_resp.header(header_name.clone(), header_value.clone());
                    }
                    client_resp.streaming(res)
                })
        })
}

fn translate_pair(host: &str, path: &str, config: &Config) -> Result<Url> {
    let out_port = config
        .domains
        .get(host)
        .ok_or_else(|| Error::HostNotFound {
            host: host.to_owned(),
        })?
        .to_owned();

    let socket_addr: SocketAddr = (OUT_IP, out_port).into();
    let url = format!("http://{}{}", socket_addr, path);
    url.parse::<Url>()
        .map_err(|source| Error::InvalidUpstreamUrl { url, source })
}

fn map_request(req: HttpRequest, config: &Config) -> Result<Url> {
    let host = req
        .headers()
        .get("host")
        .ok_or(Error::HostEmpty)?
        .to_str()
        .unwrap();

    let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");

    translate_pair(host, path, config)
}