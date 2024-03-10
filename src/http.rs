use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

#[derive(Debug, Clone)]
pub enum RequestMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: RequestMethod,
    pub path: String,
    pub version: f32,
    pub headers: Vec<(String, String)>,
}

impl HttpRequest {
    pub fn from_reader(buf: &mut BufReader<&TcpStream>) -> Result<Self, anyhow::Error> {
        let mut req_start_line = String::new();
        buf.read_line(&mut req_start_line)
            .context("Failed to read HTTP Request start line")?;

        let req_parts = req_start_line.split_whitespace().collect::<Vec<_>>();
        let method = parse_request_method(req_parts[0])?;
        let path = req_parts[1].to_string();
        let version = req_parts[2][5..]
            .parse::<f32>()
            .context("Failed to parse HTTP version from request start line")?;
        let headers = get_headers(buf).context("Failed to parse req headers")?;

        println!("{:?}", req_parts);
        Ok(HttpRequest {
            method,
            path,
            version,
            headers,
        })
    }
}

pub fn get_headers(
    buf: &mut BufReader<&TcpStream>,
) -> Result<Vec<(String, String)>, anyhow::Error> {
    let mut headers = Vec::new();
    loop {
        let mut header = String::new();
        buf.read_line(&mut header)
            .context("Failed to read HEADERS from Request")?;
        if header == "\r\n" {
            break;
        }
        // let mut header_parts = header.splitn(2, ": ");
        // let (key, val) = (
        //     header_parts
        //         .next()
        //         .ok_or_else(|| anyhow::anyhow!("Failed to parse req header key"))
        //         .unwrap()
        //         .to_string(),
        //     header_parts
        //         .next()
        //         .ok_or_else(|| anyhow::anyhow!("Failed to parse req header value"))
        //         .unwrap()
        //         .to_string(),
        // );

        if let Some((key, val)) = header.trim().split_once(": ") {
            let val = val.trim_end_matches("\r\n");
            headers.push((key.to_string(), val.to_string()))
        }

        // headers.push((key, val));
    }

    Ok(headers)
}

pub fn parse_request_method(method: &str) -> Result<RequestMethod, anyhow::Error> {
    match method {
        "GET" => Ok(RequestMethod::GET),
        "POST" => Ok(RequestMethod::POST),
        "PUT" => Ok(RequestMethod::PUT),
        "PATCH" => Ok(RequestMethod::PATCH),
        "DELETE" => Ok(RequestMethod::DELETE),
        _ => Err(anyhow::anyhow!("Invalid HTTP METHOD in request start line")),
    }
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    status_code: u16,
    status_text: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

impl HttpResponse {
    pub fn new() -> Self {
        HttpResponse {
            status_code: 200,
            status_text: "OK".to_string(),
            headers: Vec::new(),
            body: None,
        }
    }

    pub fn set_status_code(&mut self, code: u16) {
        self.status_code = code;
        match code {
            200 => self.status_text = "OK".to_string(),
            404 => self.status_text = "NOT FOUND".to_string(),
            400 => self.status_text = "BAD REQUEST".to_string(),
            401 => self.status_text = "UNAUTHORIZED".to_string(),
            _ => self.status_text = "UNKNOWN STATUS".to_string(),
        }
    }

    pub fn set_header(&mut self, key: &str, val: &str) {
        self.headers.push((key.to_string(), val.to_string()));
    }

    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body = Some(body);
    }

    pub fn append_body(&mut self, body: Vec<u8>) {
        self.body.as_mut().unwrap().extend(body);
    }

    pub fn write_to_buffer(&self) -> Result<Vec<u8>, anyhow::Error> {
        let mut buffer = Vec::new();

        // status line
        write!(
            buffer,
            "HTTP/1.1 {} {}\r\n",
            self.status_code, self.status_text
        )
        .context("Failed to write response status line")?;
        // headers
        for (key, value) in &self.headers {
            if key == "Content-Length" {
                continue;
            }
            write!(buffer, "{}: {}\r\n", key, value).context("Failed to write response header")?;
        }

        if self.body.is_some() {
            // content type
            write!(buffer, "Content-Type: text/plain\r\n")
                .context("Failed to write resonse content-type")?;
            // content length
            write!(
                buffer,
                "Content-Length: {}\r\n",
                self.body.as_ref().unwrap().len()
            )?;
            // body
            write!(buffer, "\r\n")?;
            buffer.extend_from_slice(self.body.as_ref().unwrap());
        } else {
            write!(buffer, "Content-Length: 0\r\n\r\n")
                .context("Failed to write Content-Length for empty body")?;
        }

        Ok(buffer)
    }
}

// re: clippy suggestion
impl Default for HttpResponse {
    fn default() -> Self {
        Self::new()
    }
}
