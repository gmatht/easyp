#![allow(clippy::trivial_regex)]

use lazy_static::lazy_static;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

lazy_static! {
    static ref RE_URL: regex::Regex = regex::Regex::new("<URL>").unwrap();
}

pub struct TestServer {
    pub dir_url: String,
    shutdown: Option<std::sync::mpsc::Sender<()>>,
    finalized: Arc<Mutex<bool>>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.shutdown.take().unwrap().send(()).ok();
    }
}

fn handle_connection(mut stream: TcpStream, url: &str, finalized: Arc<Mutex<bool>>) {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();
    
    let request = String::from_utf8_lossy(&buffer[..]);
    
    if request.starts_with("GET /directory") {
        let body = format!(r#"{{
    "keyChange": "{}/acme/key-change",
    "newAccount": "{}/acme/new-acct",
    "newNonce": "{}/acme/new-nonce",
    "newOrder": "{}/acme/new-order",
    "revokeCert": "{}/acme/revoke-cert",
    "meta": {{
        "caaIdentities": [
        "testdir.org"
        ]
    }}
    }}"#, url, url, url, url, url);
        
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
            body.len(),
            body
        );
        
        stream.write_all(response.as_bytes()).unwrap();
    } else if request.starts_with("HEAD /acme/new-nonce") {
        let response = "HTTP/1.1 204 No Content\r\nReplay-Nonce: 8_uBBV3N2DBRJczhoiB46ugJKUkUHxGzVe6xIMpjHFM\r\n\r\n";
        stream.write_all(response.as_bytes()).unwrap();
    } else if request.starts_with("POST /acme/new-acct") {
        let body = format!(r#"{{
    "id": 7728515,
    "key": {{
        "use": "sig",
        "kty": "EC",
        "crv": "P-256",
        "alg": "ES256",
        "x": "ttpobTRK2bw7ttGBESRO7Nb23mbIRfnRZwunL1W6wRI",
        "y": "h2Z00J37_2qRKH0-flrHEsH0xbit915Tyvd2v_CAOSk"
    }},
    "contact": [
        "mailto:foo@bar.com"
    ],
    "initialIp": "90.171.37.12",
    "createdAt": "2018-12-31T17:15:40.399104457Z",
    "status": "valid"
    }}"#);
        
        let response = format!(
            "HTTP/1.1 201 Created\r\nLocation: {}/acme/acct/7728515\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
            url, body.len(), body
        );
        
        stream.write_all(response.as_bytes()).unwrap();
    } else if request.starts_with("POST /acme/new-order") {
        let body = format!(r#"{{
    "status": "pending",
    "expires": "2019-01-09T08:26:43.570360537Z",
    "identifiers": [
        {{
        "type": "dns",
        "value": "acmetest.example.com"
        }}
    ],
    "authorizations": [
        "{}/acme/authz/YTqpYUthlVfwBncUufE8IRWLMSRqcSs"
    ],
    "finalize": "{}/acme/finalize/7738992/18234324"
    }}"#, url, url);
        
        let response = format!(
            "HTTP/1.1 201 Created\r\nLocation: {}/acme/order/YTqpYUthlVfwBncUufE8\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
            url, body.len(), body
        );
        
        stream.write_all(response.as_bytes()).unwrap();
    } else if request.starts_with("POST /acme/order/YTqpYUthlVfwBncUufE8") {
        let is_finalized = *finalized.lock().unwrap();
        let status = if is_finalized { "valid" } else { "pending" };
        
        let body = format!(r#"{{
    "status": "{}",
    "expires": "2019-01-09T08:26:43.570360537Z",
    "identifiers": [
        {{
        "type": "dns",
        "value": "acmetest.example.com"
        }}
    ],
    "authorizations": [
        "{}/acme/authz/YTqpYUthlVfwBncUufE8IRWLMSRqcSs"
    ],
    "finalize": "{}/acme/finalize/7738992/18234324",
    "certificate": "{}/acme/cert/fae41c070f967713109028"
    }}"#, status, url, url, url);
        
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
            body.len(), body
        );
        
        stream.write_all(response.as_bytes()).unwrap();
    } else if request.starts_with("POST /acme/authz/YTqpYUthlVfwBncUufE8IRWLMSRqcSs") {
        let body = format!(r#"{{
        "identifier": {{
            "type": "dns",
            "value": "acmetest.algesten.se"
        }},
        "status": "pending",
        "expires": "2019-01-09T08:26:43Z",
        "challenges": [
        {{
            "type": "http-01",
            "status": "pending",
            "url": "{}/acme/challenge/YTqpYUthlVfwBncUufE8IRWLMSRqcSs/216789597",
            "token": "MUi-gqeOJdRkSb_YR2eaMxQBqf6al8dgt_dOttSWb0w"
        }},
        {{
            "type": "tls-alpn-01",
            "status": "pending",
            "url": "{}/acme/challenge/YTqpYUthlVfwBncUufE8IRWLMSRqcSs/216789598",
            "token": "WCdRWkCy4THTD_j5IH4ISAzr59lFIg5wzYmKxuOJ1lU"
        }},
        {{
            "type": "dns-01",
            "status": "pending",
            "url": "{}/acme/challenge/YTqpYUthlVfwBncUufE8IRWLMSRqcSs/216789599",
            "token": "RRo2ZcXAEqxKvMH8RGcATjSK1KknLEUmauwfQ5i3gG8"
        }}
        ]
    }}"#, url, url, url);
        
        let response = format!(
            "HTTP/1.1 201 Created\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
            body.len(), body
        );
        
        stream.write_all(response.as_bytes()).unwrap();
    } else if request.starts_with("POST /acme/finalize/7738992/18234324") {
        // Mark as finalized
        *finalized.lock().unwrap() = true;
        
        let body = format!(r#"{{
    "status": "valid",
    "expires": "2019-01-09T08:26:43.570360537Z",
    "identifiers": [
        {{
        "type": "dns",
        "value": "acmetest.example.com"
        }}
    ],
    "authorizations": [
        "{}/acme/authz/YTqpYUthlVfwBncUufE8IRWLMSRqcSs"
    ],
    "finalize": "{}/acme/finalize/7738992/18234324",
    "certificate": "{}/acme/cert/fae41c070f967713109028"
    }}"#, url, url, url);
        
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
            body.len(), body
        );
        
        stream.write_all(response.as_bytes()).unwrap();
    } else if request.starts_with("POST /acme/cert/fae41c070f967713109028") {
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\nCERT HERE";
        stream.write_all(response.as_bytes()).unwrap();
    } else {
        let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(response.as_bytes()).unwrap();
    }
}

pub fn with_directory_server() -> TestServer {
    let tcp = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = tcp.local_addr().unwrap().port();

    let url = format!("http://127.0.0.1:{}", port);
    let dir_url = format!("{}/directory", url);

    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let finalized = Arc::new(Mutex::new(false));
    let finalized_clone = finalized.clone();

    thread::spawn(move || {
        for stream in tcp.incoming() {
            match stream {
                Ok(stream) => {
                    let url2 = url.clone();
                    let finalized2 = finalized_clone.clone();
                    thread::spawn(move || {
                        handle_connection(stream, &url2, finalized2);
                    });
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
            
            // Check if we should shutdown
            if rx.try_recv().is_ok() {
                break;
            }
        }
    });

    TestServer {
        dir_url,
        shutdown: Some(tx),
        finalized,
    }
}

#[test]
pub fn test_make_directory() {
    let server = with_directory_server();
    let res = ureq::get(&server.dir_url).call();
    assert!(res.is_ok());
}
