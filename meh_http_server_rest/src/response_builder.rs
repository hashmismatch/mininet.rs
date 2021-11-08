use std::ops::Deref;

use meh_http_common::{req::HttpServerHeader, resp::{HttpResponseWriter, HttpStatusCodes}, stack::TcpSocket};
use meh_http_server::HttpContext;

use crate::{RestError, extras::Extras, middleware::HttpMiddlewareContext};

pub struct HttpResponseBuilder<S>
where
    S: HttpMiddlewareContext,
{
    pub additional_headers: Vec<HttpServerHeader>,
    pub extras: Extras,
    pub(crate) ctx: HttpContext<S::Socket>,
}

impl<S> Deref for HttpResponseBuilder<S>
where
    S: HttpMiddlewareContext,
{
    type Target = HttpContext<S::Socket>;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl<S> HttpResponseBuilder<S>
where
    S: HttpMiddlewareContext,
{
    pub async fn response(
        mut self,
        code: HttpStatusCodes,
        content_type: Option<&str>,
        body: Option<&str>,
    ) -> Result<HttpReponseComplete, RestError> {
        let (http_code, http_code_str) = code.to_http();

        self.ctx
            .socket
            .send(format!("HTTP/1.1 {} {}\r\n", http_code, http_code_str).as_bytes())
            .await?;

        if let Some(content_type) = content_type {
            self.ctx.write(b"Content-Type: ").await?;
            self.ctx.write(content_type.as_bytes()).await?;
            self.ctx.write(b"\r\n").await?;
        }

        for header in self.additional_headers {
            self.ctx
                .write(format!("{}: {}\r\n", header.name, header.value).as_bytes())
                .await?;
        }

        self.ctx.write(b"\r\n").await?;
        if let Some(body) = body {
            self.ctx.write(body.as_bytes()).await?;
        }

        Ok(HttpReponseComplete::new())
    }
}

pub struct HttpReponseComplete {}
impl HttpReponseComplete {
    fn new() -> Self {
        HttpReponseComplete {}
    }
}
