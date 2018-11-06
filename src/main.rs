#![feature(uniform_paths, unboxed_closures, fn_traits)]
#![warn(unused)]
mod config;
use config::read_config;
use futures::future;
use futures::future::IntoFuture;
use hyper::rt::{self, Future};
use hyper::service::service_fn;
use hyper::Body;
use hyper::{Client, Request, Response, Server, Uri};
use log::{debug, trace};
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;

fn main() {
    pretty_env_logger::init();
    let in_addr = ([0, 0, 0, 0], 80).into();
    let domains_main = read_config("config.toml").expect("Unable to access config.toml");

    let client_main = Client::new();

    // new_service is run for each connection, creating a 'service'
    // to handle requests for that specific connection.
    let new_service = move || {
        let client = client_main.clone();
        // TODO: is this a leak?
        let domains = domains_main.clone();
        // This is the `Service` that will handle the connection.
        // `service_fn_ok` is a helper to convert a function that
        // returns a Response into a `Service`.
        service_fn(move |mut req| match translate_uri(&domains, &req) {
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

    let server = Server::bind(&in_addr)
        .serve(new_service)
        .map_err(|e| eprintln!("server error: {}", e));

    println!("Listening on http://{}", in_addr);

    rt::run(server);
}

fn translate_uri<T>(
    domains: &HashMap<String, u16>,
    req: &Request<T>,
) -> Result<Uri, Box<dyn Error>> {
    let domain = req
        .headers()
        .get("host")
        .ok_or("Host header not present".to_string())?
        .to_str()?;

    let port = domains
        .get(domain)
        .ok_or("Domain unrecognized".to_string())?
        .to_owned();
    let socket_addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");
    let uri_string = format!("http://{}{}", socket_addr, path,);
    uri_string
        .parse::<Uri>()
        .map_err(|e| Box::new(e) as Box<Error>)
}
