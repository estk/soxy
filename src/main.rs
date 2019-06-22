mod config;

use actix_web::client::Client;
use actix_web::error::{self, Error};
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use futures::future::{Future, IntoFuture};
use lazy_static::lazy_static;
use std::net::SocketAddr;
use url::Url;

const OUT_IP: [u8; 4] = [127, 0, 0, 1];

lazy_static! {
    static ref CONFIG: config::Config =
        config::parse_config("config.toml").expect("Unable to access config.toml");
}

fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .data(Client::new())
            .wrap(middleware::Logger::default())
            .default_service(web::route().to_async(forward))
    })
    .bind(CONFIG.listen_addr)?
    .system_exit()
    .run()
}

fn forward(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    map_request(req.clone())
        .map_err(error::ErrorNotFound)
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
                .map_err(Error::from)
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

fn translate_pair(host: &str, path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    let out_port = CONFIG
        .domains
        .get(host)
        .ok_or("Unrecognized Host")?
        .to_owned();

    let socket_addr: SocketAddr = (OUT_IP, out_port).into();
    let uri_string = format!("http://{}{}", socket_addr, path);
    uri_string
        .parse::<Url>()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

fn map_request(req: HttpRequest) -> Result<Url, Box<dyn std::error::Error>> {
    let host = req
        .headers()
        .get("host")
        .expect("Host header not present")
        .to_str()
        .unwrap();
    let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");

    translate_pair(host, path)
}