//! Example: in-process TLS client-server handshake over socketpair.
use lsb_openssl::Openssl;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixStream;

fn main() {
    let ssl = Openssl::load().expect("OpenSSL load");
    ssl.init().expect("init");
    println!("variant: {:?}, version: {:?}", ssl.variant, ssl.version());

    // Create a loopback pair and do a TLS handshake
    let (client_stream, server_stream) = UnixStream::pair().unwrap();

    let server_ctx = ssl.ctx_new(false).expect("server ctx");
    server_ctx
        .load_cert_file(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/resources/cert.pem"),
        )
        .expect("cert");
    server_ctx
        .load_key_file(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/resources/key.pem"),
        )
        .expect("key");

    let client_ctx = ssl.ctx_new(true).expect("client ctx");

    let client = ssl
        .ssl_new_from_fd(&client_ctx, client_stream.as_raw_fd())
        .expect("client ssl");
    let server = ssl
        .ssl_new_from_fd(&server_ctx, server_stream.as_raw_fd())
        .expect("server ssl");

    let client_thread = std::thread::spawn(move || {
        client.connect().expect("client handshake");
        let n = client.write(b"ping").expect("client write");
        assert_eq!(n, 4);

        let mut buf = [0u8; 1024];
        let n = client.read(&mut buf).expect("client read");
        println!("client received: {}", String::from_utf8_lossy(&buf[..n]));
        client.shutdown().ok();
    });

    let server_thread = std::thread::spawn(move || {
        server.accept().expect("server accept");
        let mut buf = [0u8; 1024];
        let n = server.read(&mut buf).expect("server read");
        println!("server received: {}", String::from_utf8_lossy(&buf[..n]));

        server.write(b"pong").expect("server write");
        server.shutdown().ok();
    });

    client_thread.join().unwrap();
    server_thread.join().unwrap();
    println!("TLS handshake test passed");
}
