// use anyhow::Context;
use anyhow::{Context, Result};
use http_server_starter_rust::http::{HttpRequest, HttpResponse};
use std::io::{BufReader, Write};
use std::net::{TcpListener, TcpStream};

fn handler(mut stream: TcpStream) -> Result<(), anyhow::Error> {
    // -- init reader + read request -- //
    let mut buf = BufReader::new(&stream);
    let request = HttpRequest::from_reader(&mut buf)?;

    let path = request.path.as_str();
    match path {
        "/" => {
            let res = HttpResponse::new();
            res.write(&mut stream)
                .context("Failed to write response to GET `/` request")?;
        }
        path if path.starts_with("/echo/") => {
            let res = echo_route(path);
            res.write(&mut stream)
                .context("Failed to write response body to GET `/echo/` request")?;
        }
        _ => {
            let mut res = HttpResponse::new();
            res.set_status_code(404);
            res.write(&mut stream)
                .context("Failed to write response for `404` NOT FOUND request")?;
        }
    }

    stream.flush().context("Failed to flush TCP stream")?;
    Ok(())
}

// -- helper re: routing -- //
pub fn echo_route(path: &str) -> HttpResponse {
    let mut res = HttpResponse::new();
    let body = path.replace("/echo/", "").as_bytes().to_vec();
    res.set_body(body);
    res
}

fn main() -> Result<()> {
    let listener =
        TcpListener::bind("127.0.0.1:4221").context("Failed to bind TCP Listener to port 4221")?;
    println!("Listening on port {}", listener.local_addr().unwrap());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                handler(stream)?;
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    Ok(())
}
