use std::collections::HashMap;
use std::io::Write;
use std::net::TcpStream;

use crate::api::ApiProblem;

pub(crate) type ReqResult<T> = std::result::Result<T, ApiProblem>;

const TIMEOUT_DURATION: std::time::Duration = std::time::Duration::from_secs(30);

#[derive(Debug)]
pub struct Body {
    data: Vec<u8>,
}

impl Body {
    pub fn read_to_string(&mut self) -> Result<String, std::io::Error> {
        Ok(String::from_utf8_lossy(&self.data).to_string())
    }
}

#[derive(Debug)]
pub struct Response {
    status: u16,
    headers: HashMap<String, String>,
    body: Body,
}

impl Response {
    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    pub fn body_mut(&mut self) -> &mut Body {
        &mut self.body
    }
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Http(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::Http(e) => write!(f, "HTTP error: {}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

fn parse_url(url: &str) -> (&str, u16, &str) {
    let rest = url.strip_prefix("https://").unwrap_or(url);
    let (host_port, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let (host, port) = match host_port.find(':') {
        Some(i) => (&host_port[..i], host_port[i+1..].parse().unwrap_or(443)),
        None => (host_port, 443),
    };
    (host, port, path)
}

fn connector() -> &'static lsb_openssl::tls::TlsConnector {
    static CONNECTOR: std::sync::OnceLock<lsb_openssl::tls::TlsConnector> = std::sync::OnceLock::new();
    CONNECTOR.get_or_init(|| {
        lsb_openssl::tls::TlsConnector::new(true)
            .expect("OpenSSL failed to load/init")
    })
}

fn connect_tls(host: &str, port: u16) -> Result<lsb_openssl::tls::TlsStream, Error> {
    let tcp = TcpStream::connect((host, port))?;
    tcp.set_read_timeout(Some(TIMEOUT_DURATION))?;
    tcp.set_write_timeout(Some(TIMEOUT_DURATION))?;

    connector().connect(tcp, host)
        .map_err(|e| Error::Http(format!("TLS handshake failed: {}", e)))
}

fn send_request(host: &str, tls: &mut lsb_openssl::tls::TlsStream, request: &str) -> Result<Vec<u8>, Error> {
    use std::io::Write;
    tls.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        match tls.read(&mut buf) {
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(e) => {
                if response.is_empty() {
                    return Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::ConnectionAborted,
                        e.to_string(),
                    )));
                }
                break;
            }
        }
        if response.len() >= 4 && &response[response.len()-4..] == b"\r\n\r\n" {
            let response_str = String::from_utf8_lossy(&response);
            if let Some(cl_line) = response_str.lines().find(|l| l.to_lowercase().starts_with("content-length:")) {
                if let Some(cl_str) = cl_line.split(':').nth(1) {
                    if let Ok(cl) = cl_str.trim().parse::<usize>() {
                        let header_end = response_str.find("\r\n\r\n").unwrap() + 4;
                        if response.len() >= header_end + cl {
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(response)
}

pub(crate) fn req_get(url: &str) -> Result<Response, Error> {
    log::trace!("GET {}", url);
    let (host, port, path) = parse_url(url);
    let mut tls = connect_tls(host, port)?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nAccept: */*\r\n\r\n",
        path, host
    );
    let response_bytes = send_request(host, &mut tls, &request)?;
    parse_http_response(response_bytes)
}

pub(crate) fn req_head(url: &str) -> Result<Response, Error> {
    log::trace!("HEAD {}", url);
    let (host, port, path) = parse_url(url);
    let mut tls = connect_tls(host, port)?;
    let request = format!(
        "HEAD {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nAccept: */*\r\n\r\n",
        path, host
    );
    let response_bytes = send_request(host, &mut tls, &request)?;
    parse_http_response(response_bytes)
}

pub(crate) fn req_post(url: &str, body: &str) -> Result<Response, Error> {
    log::trace!("POST {} {}", url, body);
    let (host, port, path) = parse_url(url);
    let mut tls = connect_tls(host, port)?;
    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/jose+json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, host, body.len(), body
    );
    let response_bytes = send_request(host, &mut tls, &request)?;
    parse_http_response(response_bytes)
}

fn parse_http_response(data: Vec<u8>) -> Result<Response, Error> {
    let response_str = String::from_utf8_lossy(&data);
    let mut lines = response_str.lines();

    let status_line = lines.next().unwrap_or("");
    let status = status_line.split(' ').nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);

    let mut headers = HashMap::new();
    let mut body_start = 0;

    for line in lines.by_ref() {
        if line.is_empty() {
            body_start = line.as_ptr() as usize - response_str.as_ptr() as usize
                + 2;
            break;
        }
        if let Some(colon) = line.find(':') {
            let key = line[..colon].trim().to_lowercase();
            let value = line[colon + 1..].trim().to_string();
            headers.insert(key, value);
        }
    }

    let body_data = if body_start > 0 && body_start <= data.len() {
        data[body_start..].to_vec()
    } else {
        Vec::new()
    };

    Ok(Response {
        status,
        headers,
        body: Body { data: body_data },
    })
}

pub(crate) fn req_handle_error(
    res: Result<Response, Error>,
) -> ReqResult<Response> {
    let res = match res {
        Ok(res) => res,
        Err(e) => {
            return Err(ApiProblem {
                _type: "httpReqError".into(),
                detail: Some(e.to_string()),
                subproblems: None,
            })
        }
    };

    if res.is_success() {
        return Ok(res);
    }

    let problem = if res.headers().get("content-type").map(|s| s.as_str()) == Some("application/problem+json") {
        let body = req_safe_read_body(res);
        serde_json::from_str(&body).unwrap_or_else(|e| ApiProblem {
            _type: "problemJsonFail".into(),
            detail: Some(format!(
                "Failed to deserialize application/problem+json ({}) body: {}",
                e.to_string(),
                body
            )),
            subproblems: None,
        })
    } else {
        let status = format!("{}", res.status());
        let body = req_safe_read_body(res);
        let detail = format!("{} body: {}", status, body);
        ApiProblem {
            _type: "httpReqError".into(),
            detail: Some(detail),
            subproblems: None,
        }
    };

    Err(problem)
}

pub(crate) fn req_expect_header(res: &Response, name: &str) -> ReqResult<String> {
    res.headers()
        .get(name)
        .ok_or_else(|| ApiProblem {
            _type: format!("Missing header: {}", name),
            detail: None,
            subproblems: None,
        })
        .map(|v| v.clone())
}

pub(crate) fn req_safe_read_body(mut res: Response) -> String {
    res.body_mut().read_to_string().unwrap_or_default()
}
