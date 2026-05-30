// cgi_env.rs - Minimal CGI environment for extensions
use std::collections::HashMap;

#[derive(Debug)]
pub struct CgiEnv {
    pub request_uri: String,
    pub query_string: String,
    pub method: String,
    pub host: String,
    pub headers: HashMap<String, String>,
}

impl CgiEnv {
    pub fn from_request(
        method: &str,
        uri: &str,
        host: &str,
        query_string: &str,
        headers: &HashMap<String, String>,
    ) -> Self {
        Self {
            request_uri: uri.to_string(),
            query_string: query_string.to_string(),
            method: method.to_string(),
            host: host.to_string(),
            headers: headers.clone(),
        }
    }

    pub fn parse_query(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();
        for pair in self.query_string.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                params.insert(key.to_string(), value.to_string());
            }
        }
        params
    }
}

pub fn url_decode(s: &str) -> String {
    // Simple URL decoding - replace %20 with space, etc.
    s.replace("%20", " ")
     .replace("%21", "!")
     .replace("%22", "\"")
     .replace("%23", "#")
     .replace("%24", "$")
     .replace("%25", "%")
     .replace("%26", "&")
     .replace("%27", "'")
     .replace("%28", "(")
     .replace("%29", ")")
     .replace("%2B", "+")
     .replace("%2C", ",")
     .replace("%2F", "/")
     .replace("%3A", ":")
     .replace("%3B", ";")
     .replace("%3C", "<")
     .replace("%3D", "=")
     .replace("%3E", ">")
     .replace("%3F", "?")
     .replace("%40", "@")
}
