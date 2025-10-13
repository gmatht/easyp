use std::net::TcpStream;
use std::io::{Read, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to localhost:8443...");
    
    let mut stream = TcpStream::connect("localhost:8443")?;
    println!("Connected successfully!");
    
    // Send a simple HTTP request
    let request = "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    stream.write_all(request.as_bytes())?;
    println!("Sent HTTP request");
    
    // Read response
    let mut buffer = [0; 1024];
    let bytes_read = stream.read(&mut buffer)?;
    let response = String::from_utf8_lossy(&buffer[..bytes_read]);
    println!("Response ({} bytes):\n{}", bytes_read, response);
    
    Ok(())
}




