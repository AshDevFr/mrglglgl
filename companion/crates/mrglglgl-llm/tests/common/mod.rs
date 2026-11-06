//! A minimal HTTP/1.1 server for tests: one handler, one response per connection, bodies sent in
//! chunks with an optional pause between them so streaming, cancellation and timeouts can be
//! exercised without the network.

#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub chunks: Vec<Vec<u8>>,
    /// Pause before each chunk after the first.
    pub pause: Duration,
}

impl Response {
    pub fn new(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            headers: Vec::new(),
            chunks: vec![body.into()],
            pause: Duration::ZERO,
        }
    }

    pub fn chunked(status: u16, chunks: Vec<Vec<u8>>, pause: Duration) -> Self {
        Self {
            status,
            headers: Vec::new(),
            chunks,
            pause,
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

pub struct MockServer {
    pub url: String,
    hits: Arc<AtomicUsize>,
}

impl MockServer {
    /// Requests received so far.
    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::SeqCst)
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        206 => "Partial Content",
        401 => "Unauthorized",
        404 => "Not Found",
        _ => "Status",
    }
}

pub async fn serve(handler: impl Fn(Request) -> Response + Send + Sync + 'static) -> MockServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handler = Arc::new(handler);
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = hits.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            counter.fetch_add(1, Ordering::SeqCst);
            let handler = handler.clone();
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                let head_end = loop {
                    let Ok(n) = socket.read(&mut chunk).await else {
                        return;
                    };
                    if n == 0 {
                        return;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    if let Some(at) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        break at;
                    }
                };
                let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
                let mut lines = head.lines();
                let first: Vec<&str> = lines.next().unwrap_or("").split(' ').collect();
                let headers: Vec<(String, String)> = lines
                    .filter_map(|l| l.split_once(':'))
                    .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
                    .collect();
                let length: usize = headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                    .and_then(|(_, v)| v.parse().ok())
                    .unwrap_or(0);
                let mut body = buf[head_end + 4..].to_vec();
                while body.len() < length {
                    let Ok(n) = socket.read(&mut chunk).await else {
                        return;
                    };
                    if n == 0 {
                        break;
                    }
                    body.extend_from_slice(&chunk[..n]);
                }
                let request = Request {
                    method: first.first().unwrap_or(&"").to_string(),
                    path: first.get(1).unwrap_or(&"").to_string(),
                    headers,
                    body: String::from_utf8_lossy(&body).to_string(),
                };
                let response = handler(request);
                let total: usize = response.chunks.iter().map(Vec::len).sum();
                let mut head = format!(
                    "HTTP/1.1 {} {}\r\nContent-Length: {total}\r\nConnection: close\r\n",
                    response.status,
                    reason(response.status)
                );
                for (k, v) in &response.headers {
                    head.push_str(&format!("{k}: {v}\r\n"));
                }
                head.push_str("\r\n");
                if socket.write_all(head.as_bytes()).await.is_err() {
                    return;
                }
                for (i, part) in response.chunks.iter().enumerate() {
                    if i > 0 && !response.pause.is_zero() {
                        tokio::time::sleep(response.pause).await;
                    }
                    if socket.write_all(part).await.is_err() || socket.flush().await.is_err() {
                        return;
                    }
                }
            });
        }
    });
    MockServer { url, hits }
}
