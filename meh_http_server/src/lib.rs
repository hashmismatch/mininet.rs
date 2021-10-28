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
use meh_http_common::{
    req::{HttpServerHeader, HttpServerRequest},
    resp::HttpResponseWriter,
    stack::{TcpError, TcpListen, TcpSocket},
};
use slog::{error, info, o, Logger};

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

pub async fn http_server<L, H, Fut>(logger: &Logger, mut listen: L, handler: H)
where
    H: Fn(HttpContext<L::TcpSocket>) -> Fut,
    Fut: Future<Output = ()>,
    L: TcpListen,
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

                let http_parse = parse(&logger, &mut socket);
                let timeout = Duration::from_secs(10);

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

                /*
                match with_timeout(&self.env, http_parse, timeout).await {
                    Ok(Ok(req)) => {
                        info!(logger, "HTTP request: {:#?}", req);

                        let ctx = HttpContext {
                            logger: logger.clone(),
                            request: req,
                            socket
                        };
                        handler(ctx).await;

                        info!(logger, "Request handler finished.");
                    },
                    Ok(Err(e)) => {
                        error!(logger, "Failed to parse the requst: {:?}", e);
                    },
                    Err(_) => {
                        error!(logger, "The incoming request timed out after {} seconds.", timeout.as_secs());
                    }
                }
                */
            }
            Err(_) => {
                error!(logger, "Listen socket stopped, shutting down.");
                break;
            }
        }
    }
}

/*
pub struct HttpServer<E> {
    logger: Logger,
    listen: ListenSocket,
    env: E
}

impl<E> HttpServer<E> where E: ExecuteEnvironment {
    pub fn new(logger: &Logger, env: E, listen: ListenSocket) -> Self {
        HttpServer {
            logger: logger.new(o!("ctx" => "http_server")),
            listen,
            env
        }
    }

    pub async fn start<H, Fut>(mut self, handler: H)
        where H: Fn(HttpContext) -> Fut, Fut: Future<Output=()>
    {
        let mut id = 1;
        info!(self.logger, "Http server listening");
        loop {
            match self.listen.accept_async().await {
                Ok((mut socket, addr)) => {
                    info!(self.logger, "Accepted a socket from {:?}", addr);

                    let logger = self.logger.new(o!("request_id" => id));
                    id += 1;

                    let http_parse = parse(&self.logger, &mut socket);
                    let timeout = Duration::from_secs(10);

                    match with_timeout(&self.env, http_parse, timeout).await {
                        Ok(Ok(req)) => {
                            info!(logger, "HTTP request: {:#?}", req);

                            let ctx = HttpContext {
                                logger: logger.clone(),
                                request: req,
                                socket
                            };
                            handler(ctx).await;

                            info!(logger, "Request handler finished.");
                        },
                        Ok(Err(e)) => {
                            error!(logger, "Failed to parse the requst: {:?}", e);
                        },
                        Err(_) => {
                            error!(logger, "The incoming request timed out after {} seconds.", timeout.as_secs());
                        }
                    }
                },
                Err(_) => {
                    error!(self.logger, "Listen socket stopped, shutting down.");
                    break;
                },
            }
        }
    }
}
*/

pub async fn parse<S>(logger: &Logger, socket: &mut S) -> Result<HttpServerRequest, HttpServerError>
where
    S: TcpSocket,
{
    let mut recv = vec![];
    loop {
        let mut buf = [0; 128];
        match socket.read(&mut buf).await {
            Ok(d) if d == 0 => {
                error!(logger, "Socket closed message received?");
                return Err(HttpServerError::Unknown);
            }
            Ok(b) => {
                recv.extend(&buf[0..b]);
            }
            Err(e) => {
                error!(logger, "Network error during parsing: {:?}", e);
                return Err(HttpServerError::Unknown);
            }
        }

        let mut headers_buffer = [httparse::EMPTY_HEADER; 60];

        let mut r = httparse::Request::new(&mut headers_buffer);
        let n = match r.parse(recv.as_slice()) {
            Ok(httparse::Status::Complete(size)) => size,
            Ok(httparse::Status::Partial) => {
                continue;
            }
            Err(e) => {
                error!(logger, "HTTP Parser error: {:?}", e);
                return Err(HttpServerError::Unknown);
            }
        };
        let body = &recv[n..];

        let req = HttpServerRequest {
            method: r.method.map(|m| m.to_string()),
            path: r.path.map(|p| p.to_string()),
            body: body.to_vec(),
            headers: headers_buffer
                .iter()
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
    S: TcpSocket + Send,
{
    async fn write(&mut self, data: &[u8]) -> Result<(), TcpError> {
        self.socket.send(data).await?;
        Ok(())
    }
}
