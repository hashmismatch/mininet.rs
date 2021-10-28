#![no_std]

extern crate alloc;

use alloc::{format, string::{String, ToString}, vec::Vec, vec};
use meh_http_common::{addr::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4}, stack::{TcpError, TcpSocket, TcpStack}};
//use embedded_nal::{IpAddr, Ipv4Addr, SocketAddr};
//use esp_at_stack::{processor::StackChannels, stack::{EspNetworkStack, NetworkStackError}};
use slog::{Logger, info};


#[derive(Debug)]
pub enum HttpClientError {
    //NetworkStackError(NetworkStackError),
    TcpError(TcpError),
    HttpParseError(httparse::Error),
    IncompleteResponse,
    MissingReponseCode,
    FailedStatusCode(u16),
    UrlParseError,
    UrlPortParseError,
    UnsupportedUrlScheme(String)
}

impl From<TcpError> for HttpClientError {
    fn from(e: TcpError) -> Self {
        Self::TcpError(e)
    }
}

impl From<httparse::Error> for HttpClientError {
    fn from(e: httparse::Error) -> Self {
        Self::HttpParseError(e)
    }
}

pub async fn http_get<S>(logger: &Logger, stack: &mut S, url: &str) -> Result<Response, HttpClientError>
    where S: TcpStack
{
    let url_parsed = meh_http_common::url::Url::parse(url).map_err(|e| HttpClientError::UrlParseError)?.1;
    info!(logger, "Url: {:?}", url_parsed);
    
    let port = match (url_parsed.port, url_parsed.scheme.as_str()) {
        (Some(port), _) => Ok(port),
        (None, "http") => Ok(80),
        _ => Err(HttpClientError::UrlPortParseError)
    }?;

    let socket_addr = match url_parsed.authority {
        meh_http_common::url::Authority::Hostname(ref h) => {
            stack.get_socket_address(&format!("{}:{}", h, port)).await?
        },
        meh_http_common::url::Authority::Ip((a, b, c, d)) => {
            let ip = Ipv4Addr::new(a, b, c, d);
            SocketAddrV4::new(ip, port).into()
        },
    };

    info!(logger, "Socket address: {:?}", socket_addr);
    
    let mut socket = stack.create_socket_connected(socket_addr).await?;
    
    let host = match url_parsed.authority {
        meh_http_common::url::Authority::Hostname(ref h) => h.clone(),
        meh_http_common::url::Authority::Ip(ip) => {
            let host: Vec<_> = [ip.0, ip.1, ip.2, ip.3].iter().map(|o| o.to_string()).collect();
            host.join(".")
        },
    };

    let path = match url_parsed.path {
        Some(p) => p,
        None => "/".into(),
    };

    let http_get = format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, host);
    info!(logger, "HTTP sending: {}", http_get);

    socket.send(http_get.as_bytes()).await?;
    
    let buf = socket.read_to_end().await?;
    info!(logger, "Received data len: {}", buf.len());

    drop(socket);

    let mut headers_buffer = [httparse::EMPTY_HEADER; 60];

    let mut r = httparse::Response::new(&mut headers_buffer);
    let n = match r.parse(buf.as_slice()) {
        Ok(httparse::Status::Complete(size)) => {
            size
        },
        Ok(httparse::Status::Partial) => {
            return Err(HttpClientError::IncompleteResponse);
        }
        Err(e) => {
            return Err(e.into());
        }
    };
    let body = &buf[n..];

    let status_code = r.code.ok_or(HttpClientError::MissingReponseCode)?;
    if (status_code >= 200 && status_code <= 299) == false {
        return Err(HttpClientError::FailedStatusCode(status_code));
    }

    info!(logger, "Headers: {:?}", r.headers);
    if let Ok(s) = alloc::str::from_utf8(body) {
        info!(logger, "Body as string: {}", s);
    }

    let headers = r.headers.iter().filter_map(|h| {
        let v = alloc::str::from_utf8(h.value);

        match v {
            Ok(v) => Some((h.name.to_string(), v.to_string())),
            _ => None
        }
    }).collect();

    let resp = Response {
        headers,
        body: body.to_vec()
    };
    Ok(resp)
}

#[derive(Debug)]
pub struct Response {
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>
}

#[derive(Debug)]
pub enum ResponseError {
    DeserializeError
}

impl Response {
    pub fn from_json<'a, T>(&'a self) -> Result<T, ResponseError>
        where T: serde::Deserialize<'a>
    {
        match serde_json::from_slice(self.body.as_slice()) {
            Ok(data) => Ok(data),
            Err(_) => Err(ResponseError::DeserializeError)
        }
    }
}