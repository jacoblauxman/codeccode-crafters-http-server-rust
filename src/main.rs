use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

const RESPONSE_OK: &str = "HTTP/1.1 200 OK\r\n\r\n";
const RESPONSE_NOTFOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";

fn handler(mut stream: TcpStream) -> Result<(), anyhow::Error> {
    // -- reading --
    let buf = BufReader::new(&mut stream);
    let req: Vec<_> = buf
        .lines()
        .map(|res| res.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    let start_line = req[0].split_whitespace().collect::<Vec<_>>();
    let (_method, path, _ver) = (start_line[0], start_line[1], start_line[2]);

    // writing
    match path {
        "/" => stream
            .write_all(RESPONSE_OK.as_bytes())
            .context("Failed to write OK response")?,
        _ => stream
            .write_all(RESPONSE_NOTFOUND.as_bytes())
            .context("Failed to write NOT FOUND response")?,
    }

    Ok(())
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
