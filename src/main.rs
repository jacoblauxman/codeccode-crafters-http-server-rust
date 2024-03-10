use anyhow::{Context, Result};
use http_server_starter_rust::http::{HttpRequest, HttpResponse};
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

async fn handler(mut stream: TcpStream) -> Result<(), anyhow::Error> {
    // -- init reader + read request -- //
    // let mut buf = BufReader::new(&stream);
    let mut buf = BufReader::new(&mut stream);
    let request = HttpRequest::from_reader(&mut buf).await?;
    let path = request.path.as_str();
    let mut _res_buffer = Vec::new();

    // access headers -- specifically for `/user-agent`
    let headers = request.headers;
    let mut user_agent = String::new();

    if let Some(ua) = headers.get("User-Agent") {
        user_agent = ua.to_string();
    }

    // 'routing'
    match path {
        "/" => {
            let res = HttpResponse::new();
            _res_buffer = res
                .write_to_buffer()
                .context("Failed to write HTTP response from `/` path to buffer")?;
            println!("{:?}", res);
        }
        path if path.starts_with("/echo/") => {
            let res = echo_route(path);
            _res_buffer = res
                .write_to_buffer()
                .context("Failed to write HTTP response from `/echo/` endpoint to buffer")?;
            println!("{:?}", res);
        }
        "/user-agent" => {
            let res = user_agent_route(user_agent);
            _res_buffer = res
                .write_to_buffer()
                .context("Failed to write HTTP response from `/user_agent` endpoint to buffer")?;
            println!("{:?}", res);
        }
        _ => {
            let mut res = HttpResponse::new();
            res.set_status_code(404);
            _res_buffer = res
                .write_to_buffer()
                .context("Failed to write HTTP response for unknown route endpoint")?;
            println!("{:?}", res);
        }
    }

    // write response buffer to stream
    stream
        .write_all(&_res_buffer)
        .await
        .context("Failed to write response to TCP stream")?;
    stream
        .flush()
        .await
        .context("Failed to flush TPCP stream")?;

    Ok(())
}

// -- HELPERS re: path / endpoints -- //
pub fn echo_route(path: &str) -> HttpResponse {
    let mut res = HttpResponse::new();
    let body = path.replace("/echo/", "").as_bytes().to_vec();
    res.set_body(body);
    res
}

pub fn user_agent_route(user_agent: String) -> HttpResponse {
    let mut res = HttpResponse::new();
    res.set_body(user_agent.into_bytes());
    res
}

//

//

//

#[tokio::main]
// async fn main() -> Result<(), anyhow::Error> {
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221")
        .await
        .context("Failed to bind TCP Listener to port 4221")?;

    loop {
        let (stream, _addr) = listener
            .accept()
            .await
            .context("Failed to establish stream to TCP listener")?;
        // println!("TcpStream connected at address: {:?}", addr);

        tokio::spawn(async move {
            if let Err(err) = handler(stream).await {
                println!("Error handling connection: {}", err);
            }
        });
    }

    // Ok(())
}
