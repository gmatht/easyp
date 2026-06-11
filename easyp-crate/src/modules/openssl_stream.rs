use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use async_io::Async;
use futures::io::{AsyncRead, AsyncWrite};
use std::net::TcpStream;

/// SSL_get_error return values
const SSL_ERROR_WANT_READ: i32 = 2;
const SSL_ERROR_WANT_WRITE: i32 = 3;

/// Async wrapper around lsb_openssl::SslConn using async-io readiness.
pub struct TlsStream {
    stream: Async<TcpStream>,
    ssl: Box<lsb_openssl::SslConn>,
}

unsafe impl Send for TlsStream {}

impl TlsStream {
    /// Create from an Async<TcpStream> and an already-handshaked SslConn (non-blocking mode set).
    pub fn new(stream: Async<TcpStream>, ssl: lsb_openssl::SslConn) -> Self {
        Self { stream, ssl: Box::new(ssl) }
    }
}

impl AsyncRead for TlsStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        match this.ssl.read(buf) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(e) => {
                let need_read = matches!(&e, lsb_openssl::SslError::Ssl(code, _) if *code == SSL_ERROR_WANT_READ);
                let need_write = matches!(&e, lsb_openssl::SslError::Ssl(code, _) if *code == SSL_ERROR_WANT_WRITE);
                let mut fut: Pin<Box<dyn Future<Output=io::Result<()>>>> = if need_read {
                    Box::pin(this.stream.readable())
                } else if need_write {
                    Box::pin(this.stream.writable())
                } else {
                    return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e.to_string())));
                };
                match fut.as_mut().poll(cx) {
                    Poll::Ready(Ok(())) => Poll::Pending,
                    Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
}

impl AsyncWrite for TlsStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        match this.ssl.write(buf) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(e) => {
                let need_read = matches!(&e, lsb_openssl::SslError::Ssl(code, _) if *code == SSL_ERROR_WANT_READ);
                let need_write = matches!(&e, lsb_openssl::SslError::Ssl(code, _) if *code == SSL_ERROR_WANT_WRITE);
                let mut fut: Pin<Box<dyn Future<Output=io::Result<()>>>> = if need_read {
                    Box::pin(this.stream.readable())
                } else if need_write {
                    Box::pin(this.stream.writable())
                } else {
                    return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e.to_string())));
                };
                match fut.as_mut().poll(cx) {
                    Poll::Ready(Ok(())) => Poll::Pending,
                    Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let _ = this.ssl.shutdown();
        Poll::Ready(Ok(()))
    }
}
