#![no_std]

extern crate alloc;

use core::time::Duration;

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use async_trait::async_trait;
use futures::Future;
use mininet_base::{req::{HttpServerHeader, HttpServerRequest}, resp::HttpResponseWriter, stack::{SystemEnvironment, TcpError, TcpListen, TcpSocket, with_timeout}};
use slog::{Logger, debug, error, info, o};

#[derive(Debug, Copy, Clone)]
pub enum HttpServerError {
    Unknown,
    TcpError(TcpError),
}

impl From<TcpError> for HttpServerError {
    fn from(v: TcpError) -> Self {
        Self::TcpError(v)
    }
}

pub async fn http_server<L, H, Fut, E>(logger: &Logger, env: E, mut listen: L, handler: H, request_timeout: Option<Duration>)
where
    H: Fn(HttpContext<L::TcpSocket>) -> Fut,
    Fut: Future<Output = ()>,
    L: TcpListen,
    E: SystemEnvironment
{
    let logger = logger.new(o!("ctx" => "http_server"));

    let mut id: usize = 1;
    info!(logger, "Http server listening");
    loop {
        match listen.accept().await {
            Ok((mut socket, addr)) => {
                info!(logger, "Accepted a socket from {:?}", addr);

                let logger = logger.new(o!("request_id" => id));
                id += 1;

                let handle_request = async {
                    let http_parse = parse(&logger, &mut socket);

                    match http_parse.await {
                        Ok(req) => {
                            info!(logger, "HTTP request: {:#?}", req);

                            let ctx = HttpContext {
                                logger: logger.clone(),
                                request: req,
                                socket,
                            };
                            handler(ctx).await;

                            info!(logger, "Request handler finished.");
                        }
                        Err(e) => {
                            error!(logger, "Failed to parse the requst: {:?}", e);
                        }
                    }
                };

                if let Some(t) = request_timeout {
                    match with_timeout(&env, handle_request, t).await {
                        Ok(_) => (),
                        Err(_) => {
                            error!(logger, "The incoming request timed out after {} seconds.", t.as_secs());
                        }
                    }
                } else {
                    handle_request.await;
                }
            }
            Err(_) => {
                error!(logger, "Listen socket stopped, shutting down.");
                break;
            }
        }
    }
}


pub async fn parse<S>(logger: &Logger, socket: &mut S) -> Result<HttpServerRequest, HttpServerError>
where
    S: TcpSocket,
{
    let mut recv_header = vec![];
    loop {
        let mut buf = [0; 64];        
        debug!(logger, "Reading header data");
        let incomplete = match socket.read(&mut buf).await {
            Ok(d) if d == 0 => {
                error!(logger, "Socket closed message received?");
                return Err(HttpServerError::Unknown);
            }
            Ok(b) => {
                recv_header.extend(&buf[0..b]);
                debug!(logger, "Received {} bytes of header data", b);
            
                b < buf.len()
            }
            Err(e) => {
                error!(logger, "Network error during parsing: {:?}", e);
                return Err(HttpServerError::Unknown);
            }
        };

        if incomplete {
            debug!(logger, "Incomplete last read?");
        }

        let mut headers_buffer = [httparse::EMPTY_HEADER; 60];

        let mut r = httparse::Request::new(&mut headers_buffer);
        let n = match r.parse(recv_header.as_slice()) {
            Ok(httparse::Status::Complete(size)) => size,
            Ok(httparse::Status::Partial) => {
                debug!(logger, "Partial headers, getting more data");
                continue;
            }
            Err(e) => {
                error!(logger, "HTTP Parser error: {:?}", e);
                return Err(HttpServerError::Unknown);
            }
        };

        let method = r.method.map(|m| m.to_string());
        let path = r.path.map(|p| p.to_string());

        let body_size = headers_buffer.iter()
            .filter(|h| h.name == "Content-Length")
            .flat_map(|h| core::str::from_utf8(h.value))
            .flat_map(|v| v.parse::<usize>())
            .next();

        let mut body = recv_header[n..].to_vec();

        // read in the remaining body, if any
        if let Some(body_size) = body_size {
            debug!(logger, "Request body size: {}", body_size);
            let mut remaining = body_size - body.len(); // of by one?

            loop {
                if remaining == 0 {
                    debug!(logger, "Whole body received.");
                    break;
                }
                
                let b = remaining.min(buf.len());
                debug!(logger, "Reading {} bytes of additional body data", b);
                match socket.read(&mut buf[0..b]).await {
                    Ok(d) if d == 0 => {
                        error!(logger, "Socket closed message received?");
                        return Err(HttpServerError::Unknown);
                    }
                    Ok(b) => {
                        body.extend(&buf[0..b]);
                        remaining -= b;
                    }
                    Err(e) => {
                        error!(logger, "Network error during body receive: {:?}", e);
                        return Err(HttpServerError::Unknown);
                    }
                }
            }
        }        

        let req = HttpServerRequest {
            method,
            path,
            body,
            headers: headers_buffer
                .iter()
                .filter(|&h| *h != httparse::EMPTY_HEADER)
                .filter_map(|h| {
                    if let Ok(val) = core::str::from_utf8(h.value) {
                        Some(HttpServerHeader {
                            name: h.name.to_string(),
                            value: val.to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect(),
        };

        return Ok(req);
    }
}

pub struct HttpContext<S>
where
    S: TcpSocket,
{
    pub logger: Logger,
    pub request: HttpServerRequest,
    pub socket: S,
}

#[async_trait]
impl<S> HttpResponseWriter for HttpContext<S>
where
    S: TcpSocket,
{
    async fn write(&mut self, data: &[u8]) -> Result<(), TcpError> {
        self.socket.send(data).await?;
        Ok(())
    }
}
