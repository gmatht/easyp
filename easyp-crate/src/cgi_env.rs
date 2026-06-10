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

pub fn url_decode(s: &str) -> Result<String, String> {
    let mut result = String::new();
    let mut chars = s.chars();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex1 = chars.next().ok_or("Invalid URL encoding")?;
            let hex2 = chars.next().ok_or("Invalid URL encoding")?;

            let hex_str = format!("{}{}", hex1, hex2);
            let byte = u8::from_str_radix(&hex_str, 16)
                .map_err(|_| "Invalid hex in URL encoding")?;

            result.push(byte as char);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }

    Ok(result)
}
