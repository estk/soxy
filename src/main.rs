#![feature(uniform_paths, unboxed_closures, fn_traits)]
#![warn(unused)]
mod config;
use config::read_config;
use hyper::rt::{self, Future};
use hyper::service::service_fn;
use hyper::{Client, Server};
use log::trace;
use std::net::SocketAddr;

fn main() {
    pretty_env_logger::init();
    let in_addr = ([0, 0, 0, 0], 80).into();
    let domains = read_config("config.toml").expect("Unable to access config.toml");

    let client_main = Client::new();

    // new_service is run for each connection, creating a 'service'
    // to handle requests for that specific connection.
    let new_service = move || {
        let client = client_main.clone();
        // TODO: is this a leak?
        let dref = domains.clone();
        // This is the `Service` that will handle the connection.
        // `service_fn_ok` is a helper to convert a function that
        // returns a Response into a `Service`.
        service_fn(move |mut req| {
            let domain = req
                .headers()
                .get("host")
                .expect("must have host header")
                .to_str()
                .expect("must have host header");
            // if failure, try *.<domain>
            trace!("domain: {}", domain);

            let port = dref
                .get(domain)
                .expect("domain not found in proxy list")
                .to_owned();
            let sock_addr: SocketAddr = ([127, 0, 0, 1], port).into();
            trace!("proxying req to {} to {}", domain, sock_addr);

            let uri_string = format!(
                "http://{}/{}",
                sock_addr,
                req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("")
            );
            let uri = uri_string.parse().unwrap();
            *req.uri_mut() = uri;
            client.request(req)
        })
    };

    let server = Server::bind(&in_addr)
        .serve(new_service)
        .map_err(|e| eprintln!("server error: {}", e));

    println!("Listening on http://{}", in_addr);

    rt::run(server);
}
