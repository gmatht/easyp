// cgi_env.rs - Minimal CGI environment for extensions
use std::collections::HashMap;

#[derive(Debug)]
pub struct CgiEnv {
    pub query_string: String,
}

impl CgiEnv {
    pub fn from_request(query_string: &str) -> Self {
        Self {
            query_string: query_string.to_string(),
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
