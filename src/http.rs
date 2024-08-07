use anyhow::{Context, Result};
use flate2::{write::GzEncoder, Compression};
use std::path::PathBuf;
use std::{collections::HashMap, io::Write};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt};

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
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

impl HttpRequest {
    // pub fn from_reader(buf: &mut BufReader<&TcpStream>) -> Result<Self, anyhow::Error> {
    pub async fn from_reader<R: AsyncBufRead + Unpin>(buf: &mut R) -> Result<Self, anyhow::Error> {
        let mut req_start_line = String::new();
        buf.read_line(&mut req_start_line)
            .await
            .context("Failed to read HTTP Request start line")?;

        let req_parts = req_start_line.split_whitespace().collect::<Vec<_>>();
        let method = parse_request_method(req_parts[0])
            .await
            .context("Failed to parse method from HTTP Request")?;
        let path = req_parts[1].to_string();

        let version = req_parts[2][5..]
            .parse::<f32>()
            .context("Failed to parse HTTP version from request start line")?;

        let headers = get_headers(buf)
            .await
            .context("Failed to parse req headers")?;

        let body = match method {
            RequestMethod::POST => {
                let mut body = Vec::new();
                if let Some(content_length) = headers.get("Content-Length") {
                    let content_length: usize = content_length
                        .parse()
                        .context("Failed to parse Content-Length header")?;

                    body.resize(content_length, 0);
                    buf.read_exact(&mut body)
                        .await
                        .context("Failed to read request body")?;
                }

                Some(body)
            }

            RequestMethod::GET => None, // no req. body for `GET`

            _ => todo!(), // still need to implement `DELETE` and `PUT/PATCH` methods
        };

        let req = HttpRequest {
            method,
            path,
            version,
            headers,
            body,
        };

        Ok(req)
    }
}

pub async fn get_headers<R: AsyncBufRead + Unpin>(
    // buf: &mut BufReader<&TcpStream>,
    buf: &mut R,
) -> Result<HashMap<String, String>, anyhow::Error> {
    let mut headers = HashMap::new();
    loop {
        let mut header = String::new();
        buf.read_line(&mut header)
            .await
            .context("Failed to read HEADERS from Request")?;
        if header == "\r\n" {
            break;
        }

        if let Some((key, val)) = header.trim().split_once(": ") {
            let val = val.trim_end_matches("\r\n");
            headers.insert(key.to_string(), val.to_string());
        }
    }

    Ok(headers)
}

pub async fn parse_request_method(method: &str) -> Result<RequestMethod, anyhow::Error> {
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
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

impl HttpResponse {
    pub fn new() -> Self {
        HttpResponse {
            status_code: 200,
            status_text: "OK".to_string(),
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn set_status_code(&mut self, code: u16) {
        self.status_code = code;
        match code {
            200 => self.status_text = "OK".to_string(),
            201 => self.status_text = "Created".to_string(),
            404 => self.status_text = "Not Found".to_string(),
            400 => self.status_text = "Bad Request".to_string(),
            401 => self.status_text = "Unauthorized".to_string(),
            _ => self.status_text = "UNKNOWN STATUS".to_string(),
        }
    }

    pub fn set_header(&mut self, key: &str, val: &str) {
        self.headers.insert(key.to_string(), val.to_string());
    }

    pub fn set_content_type(&mut self, content_type: ContentType) {
        self.headers
            .insert("Content-Type".to_string(), content_type.to_string()); // simplified via ToString impl
    }

    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body = Some(body);
    }

    pub fn append_body(&mut self, body: Vec<u8>) {
        self.body.as_mut().unwrap().extend(body);
    }

    pub async fn set_file_content(
        &mut self,
        dir_path: &PathBuf,
        file_path: &str,
    ) -> Result<(), anyhow::Error> {
        let path = PathBuf::from(dir_path).join(file_path);
        let data = tokio::fs::read(path)
            .await
            // .context("Failed to read data from given file path")?; // instead set response for 404 Not Found
            .map_err(|_err| {
                self.set_status_code(404);
            });

        if let Ok(data) = data {
            self.set_body(data);
            self.set_content_type(ContentType::OctetStream);
        }

        Ok(())
    }

    // formatting + writing res
    pub fn write_to_buffer(&self) -> Result<Vec<u8>, anyhow::Error> {
        let mut res_buffer = Vec::new();

        // status line
        res_buffer.extend_from_slice(
            format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_text).as_bytes(),
        );

        // headers
        for (key, value) in &self.headers {
            if key == "Content-Length" {
                continue;
            }

            res_buffer.extend_from_slice(format!("{}: {}\r\n", key, value).as_bytes());
        }

        // check for body content
        if self.body.is_some() {
            // content type (default)
            if self.headers.get("Content-Type").is_none() {
                res_buffer.extend_from_slice("Content-Type: text/plain\r\n".as_bytes());
            }

            match self.headers.get("Content-Encoding") {
                Some(_) => {
                    // encoding
                    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                    encoder.write_all(self.body.as_ref().unwrap())?;
                    let enc_buf = encoder.finish()?;

                    res_buffer.extend_from_slice(
                        format!("Content-Length: {}\r\n", enc_buf.len()).as_bytes(),
                    );
                    res_buffer.extend_from_slice("\r\n".as_bytes());
                    res_buffer.extend_from_slice(&enc_buf);
                }
                None => {
                    res_buffer.extend_from_slice(
                        format!("Content-Length: {}\r\n", self.body.as_ref().unwrap().len())
                            .as_bytes(),
                    );

                    res_buffer.extend_from_slice("\r\n".as_bytes());
                    res_buffer.extend_from_slice(self.body.as_ref().unwrap());
                }
            }
        } else {
            // no body, write EOF / CRLF
            res_buffer.extend_from_slice("Content-Length: 0\r\n\r\n".as_bytes());
        }

        Ok(res_buffer)
    }
}

// re: clippy suggestion
impl Default for HttpResponse {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum ContentType {
    TextPlain,
    OctetStream,
}

impl ToString for ContentType {
    fn to_string(&self) -> String {
        match self {
            ContentType::TextPlain => "text/plain".to_string(),
            ContentType::OctetStream => "application/octet-stream".to_string(),
        }
    }
}
