use std::collections::HashMap;

use crate::api::ApiProblem;

pub(crate) type ReqResult<T> = std::result::Result<T, ApiProblem>;

const TIMEOUT_DURATION: std::time::Duration = std::time::Duration::from_secs(30);

// Compatibility layer for minreq to match ureq API
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
    
    pub fn body(&self) -> &Body {
        &self.body
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

pub(crate) fn req_get(url: &str) -> Result<Response, Error> {
    trace!("GET {}", url);
    let response = minreq::get(url)
        .with_timeout(TIMEOUT_DURATION.as_secs() as u64)
        .send()
        .map_err(|e| Error::Http(e.to_string()))?;
    
    let mut headers = HashMap::new();
    for (key, value) in response.headers.iter() {
        headers.insert(key.to_lowercase(), value.to_string());
    }
    
    Ok(Response {
        status: response.status_code as u16,
        headers,
        body: Body {
            data: response.as_bytes().to_vec(),
        },
    })
}

pub(crate) fn req_head(url: &str) -> Result<Response, Error> {
    trace!("HEAD {}", url);
    let response = minreq::head(url)
        .with_timeout(TIMEOUT_DURATION.as_secs() as u64)
        .send()
        .map_err(|e| Error::Http(e.to_string()))?;
    
    let mut headers = HashMap::new();
    for (key, value) in response.headers.iter() {
        headers.insert(key.to_lowercase(), value.to_string());
    }
    
    Ok(Response {
        status: response.status_code as u16,
        headers,
        body: Body {
            data: Vec::new(), // HEAD requests have no body
        },
    })
}

pub(crate) fn req_post(url: &str, body: &str) -> Result<Response, Error> {
    trace!("POST {} {}", url, body);
    let response = minreq::post(url)
        .with_header("content-type", "application/jose+json")
        .with_timeout(TIMEOUT_DURATION.as_secs() as u64)
        .with_body(body)
        .send()
        .map_err(|e| Error::Http(e.to_string()))?;
    
    let mut headers = HashMap::new();
    for (key, value) in response.headers.iter() {
        headers.insert(key.to_lowercase(), value.to_string());
    }
    
    Ok(Response {
        status: response.status_code as u16,
        headers,
        body: Body {
            data: response.as_bytes().to_vec(),
        },
    })
}

pub(crate) fn req_handle_error(
    res: Result<Response, Error>,
) -> ReqResult<Response> {
    let res = match res {
        // ok responses pass through
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
        // if we were sent a problem+json, deserialize it
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
        // some other problem
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
    // letsencrypt sometimes closes the TLS abruptly causing io error
    // even though we did capture the body.
    let res_body = res.body_mut().read_to_string().unwrap_or_default();
    res_body
}
