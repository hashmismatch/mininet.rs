use std::borrow::Cow;

use async_trait::async_trait;

use crate::stack::TcpError;

/*
pub trait HttpResponseWriter {
    type WriteOutput: core::future::Future<Output = ()>;

    fn write(&mut self, data: &[u8]) -> Self::WriteOutput;
}
*/

#[derive(Copy, Clone, Debug)]
pub enum HttpStatusCodes {
    Ok = 200,
    Accepted = 202,
    NoContent = 204,
    BadRequest = 400,
    NotFound = 404,
    InternalError = 500
}

impl HttpStatusCodes {
    pub fn to_http(&self) -> (u16, &'static str) {
        let t = match *self {
            HttpStatusCodes::Ok => "OK",
            HttpStatusCodes::BadRequest => "Bad Request",
            HttpStatusCodes::NotFound => "Not Found",
            HttpStatusCodes::InternalError => "Internal Server Error",
            HttpStatusCodes::Accepted => "Accepted",
            HttpStatusCodes::NoContent => "No Content",
        };

        (*self as u16, t)
    }
}

impl Into<HttpStatusCode> for HttpStatusCodes {
    fn into(self) -> HttpStatusCode {
        HttpStatusCode::Standard(self)
    }
}

#[derive(Clone, Debug)]
pub enum HttpStatusCode {
    Standard(HttpStatusCodes),
    Custom(u16, Cow<'static, str>)
}

impl HttpStatusCode {
    pub fn to_http(&self) -> (u16, Cow<'static, str>) {
        match self {
            HttpStatusCode::Standard(s) => {
                let (c, s) = s.to_http();
                (c, s.into())
            },
            HttpStatusCode::Custom(c, s) => (*c, s.clone())
        }
    }
}

#[async_trait]
pub trait HttpResponseWriter where Self: Sized {
    async fn write(&mut self, data: &[u8]) -> Result<(), TcpError>;

    async fn http_reply(mut self, code: HttpStatusCode, content_type: &str, body: &str) -> Result<(), TcpError> {
        let (http_code, http_code_str) = code.to_http();

        self.write(format!("HTTP/1.1 {} {}\r\n", http_code, http_code_str).as_bytes()).await?;
        self.write(b"Content-Type: ").await?;
        self.write(content_type.as_bytes()).await?;
        self.write(b"\r\n\r\n").await?;
        self.write(body.as_bytes()).await?;

        Ok(())
    }

    async fn http_ok(mut self, content_type: &str, body: &str) -> Result<(), TcpError> {
        self.write(b"HTTP/1.1 200 OK\r\n").await?;
        self.write(b"Content-Type: ").await?;
        self.write(content_type.as_bytes()).await?;
        self.write(b"\r\n\r\n").await?;
        self.write(body.as_bytes()).await?;

        Ok(())
    }
}