#![feature(uniform_paths, unboxed_closures, fn_traits)]
#![warn(unused)]
mod config;
use config::read_domains;
use futures::future::{self, IntoFuture};
use hyper::rt::{self, Future};
use hyper::service::service_fn;
use hyper::Body;
use hyper::{Client, Request, Response, Server, Uri};
use lazy_static::lazy_static;
use log::debug;
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;

const PORT: u16 = 80;
const OUT_IP: [u8; 4] = [127, 0, 0, 1];

lazy_static! {
    static ref LISTEN_ADDR: SocketAddr = ([0, 0, 0, 0], PORT).into();
    static ref DOMAINS: HashMap<String, u16> =
        read_domains("config.toml").expect("Unable to access config.toml");
}

fn main() {
    pretty_env_logger::init();
    let client_main = Client::new();

    // new_service is run for each connection, creating a 'service'
    // to handle requests for that specific connection.
    let new_service = move || {
        let client = client_main.clone();
        // This is the `Service` that will handle the connection.
        // `service_fn_ok` is a helper to convert a function that
        // returns a Response into a `Service`.
        service_fn(move |mut req| match translate_uri(&req) {
            Err(err) => {
                debug!("{}", err);
                let res = Response::builder()
                    .status(400)
                    .body(Body::default())
                    .unwrap();
                Box::new(future::ok(res))
                    as Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>
            }
            Ok(uri) => {
                *req.uri_mut() = uri;
                Box::new(client.request(req).into_future())
            }
        })
    };

    let server = Server::bind(&LISTEN_ADDR)
        .serve(new_service)
        .map_err(|e| eprintln!("server error: {}", e));

    println!("Listening on http://{}", *LISTEN_ADDR);
    rt::run(server);
}

fn translate_uri<T>(req: &Request<T>) -> Result<Uri, Box<dyn Error>> {
    let domain = req
        .headers()
        .get("host")
        .ok_or("Host header not present".to_string())?
        .to_str()?;

    let port = DOMAINS
        .get(domain)
        .ok_or("Domain unrecognized".to_string())?
        .to_owned();

    let socket_addr: SocketAddr = (OUT_IP, port).into();
    let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");
    let uri_string = format!("http://{}{}", socket_addr, path);
    uri_string
        .parse::<Uri>()
        .map_err(|e| Box::new(e) as Box<Error>)
}
