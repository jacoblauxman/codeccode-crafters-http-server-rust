use std::io::{Read, Result, Write};
use std::net::{TcpListener, TcpStream};

fn handler(mut stream: TcpStream) -> Result<()> {
    // reading
    let mut buf = vec![];
    let _ = stream.read(&mut buf).unwrap();

    // writing
    stream.write_all("HTTP/1.1 200 OK\r\n\r\n".as_bytes())
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    println!("Listening on port {}", listener.local_addr().unwrap());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                handler(stream).unwrap();
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
