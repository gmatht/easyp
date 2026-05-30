//! Integration test: full TLS handshake over a Unix socket pair.
use lsb_openssl::{Openssl, SslConn, SslError};
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixStream;

fn tls_pair(ssl: &Openssl) -> Result<(SslConn, SslConn, UnixStream, UnixStream), SslError> {
    let (client_stream, server_stream) =
        UnixStream::pair().expect("socketpair should work");
    let (cfd, sfd) = (client_stream.as_raw_fd(), server_stream.as_raw_fd());

    let server_ctx = ssl.ctx_new(false)?;
    let cert = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/resources/cert.pem");
    let key = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/resources/key.pem");
    server_ctx.load_cert_file(cert)?;
    server_ctx.load_key_file(key)?;

    let client_ctx = ssl.ctx_new(true)?;

    let client_conn = ssl.ssl_new_from_fd(&client_ctx, cfd)?;
    let server_conn = ssl.ssl_new_from_fd(&server_ctx, sfd)?;

    Ok((client_conn, server_conn, client_stream, server_stream))
}

#[test]
fn test_tls_handshake_and_data_exchange() {
    let ssl = Openssl::load().expect("OpenSSL should load");
    ssl.init().expect("init");

    let (client_conn, server_conn, _client_stream, _server_stream) =
        tls_pair(&ssl).expect("pair creation");

    // Perform handshake in separate threads (concurrent connect/accept)
    let client_handle = std::thread::spawn(move || {
        client_conn.connect()?;
        let msg = b"hello from client";
        let n = client_conn.write(msg)?;
        assert_eq!(n, msg.len());

        let mut buf = vec![0u8; 1024];
        let n = client_conn.read(&mut buf)?;
        assert_eq!(&buf[..n], b"pong");
        client_conn.shutdown()?;
        Ok::<_, SslError>(())
    });

    let server_handle = std::thread::spawn(move || {
        server_conn.accept()?;
        let mut buf = vec![0u8; 1024];
        let n = server_conn.read(&mut buf)?;
        assert_eq!(&buf[..n], b"hello from client");

        let n = server_conn.write(b"pong")?;
        assert_eq!(n, 4);
        server_conn.shutdown()?;
        Ok::<_, SslError>(())
    });

    client_handle.join().expect("client thread panicked").expect("client handshake failed");
    server_handle.join().expect("server thread panicked").expect("server handshake failed");
}
