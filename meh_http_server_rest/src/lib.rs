pub mod quick_rest;

use std::convert::Infallible;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::Deref;
use std::pin::Pin;

use futures::pin_mut;
use meh_http_common::req::HttpServerHeader;
use meh_http_common::stack::{TcpError, TcpSocket};
use meh_http_common::resp::{HttpResponseWriter, HttpStatusCodes};
use meh_http_server::HttpContext;
use async_trait::async_trait;


pub struct HttpResponseBuilder<S>
    where S: TcpSocket
{
    pub additional_headers: Vec<HttpServerHeader>,
    ctx: HttpContext<S>
}

impl<S> Deref for  HttpResponseBuilder<S>
where S: TcpSocket
{
    type Target = HttpContext<S>;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl<S> HttpResponseBuilder<S>
    where S: TcpSocket
{
    pub async fn response(mut self, code: HttpStatusCodes, content_type: Option<&str>, body: Option<&str>) -> Result<HttpReponseComplete, RestError> {
        let (http_code, http_code_str) = code.to_http();

        self.ctx.socket.send(format!("HTTP/1.1 {} {}\r\n", http_code, http_code_str).as_bytes()).await?;

        if let Some(content_type) = content_type {
            self.ctx.write(b"Content-Type: ").await?;
            self.ctx.write(content_type.as_bytes()).await?;
            self.ctx.write(b"\r\n").await?;
        }

        for header in self.additional_headers {
            self.ctx.write(format!("{}: {}\r\n", header.name, header.value).as_bytes()).await?;
        }

        self.ctx.write(b"\r\n").await?;
        if let Some(body) = body {
            self.ctx.write(body.as_bytes()).await?;
        }

        Ok(HttpReponseComplete::new())
    }
}



pub struct HttpReponseComplete { }
impl HttpReponseComplete {
    fn new() -> Self {
        HttpReponseComplete { }
    }
}

#[async_trait]
pub trait HttpMiddleware: Send + Sized {
    type Socket: TcpSocket;

    async fn process(self, ctx: HttpContext<Self::Socket>) -> HandlerResult<Self::Socket> {
        let resp_builder = HttpResponseBuilder {
            additional_headers: vec![],
            ctx
        };

        self.handle(resp_builder).await
    }

    async fn handle(self, mut ctx: HttpResponseBuilder<Self::Socket>) -> HandlerResult<Self::Socket>;
}

pub struct HttpMidlewareChain<A, B, S>
    where 
        A: HttpMiddleware<Socket=S>,
        B: HttpMiddleware<Socket=S>,
        S: TcpSocket
{
    a: A,
    b: B,
    _socket: PhantomData<S>
}


impl<A, B, S> HttpMidlewareChain<A, B, S>
    where 
        A: HttpMiddleware<Socket=S>,
        B: HttpMiddleware<Socket=S>,
        S: TcpSocket
{
    pub fn new(a: A, b: B) -> Self {
        HttpMidlewareChain {
            a,
            b,
            _socket: PhantomData::default()
        }
    }
}


#[async_trait]
impl<A, B, S> HttpMiddleware for HttpMidlewareChain<A, B, S>
    where 
        A: HttpMiddleware<Socket=S>,
        B: HttpMiddleware<Socket=S>,
        S: TcpSocket
{
    type Socket = S;

    async fn handle(self, ctx: HttpResponseBuilder<S>) -> HandlerResult<S> {
        let res_a = self.a.handle(ctx).await;
        match res_a {            
            HandlerResult::Pass(ctx) => {
                self.b.handle(ctx).await
            },
            _ => res_a
        }
    }
}


pub struct HttpMidlewareFn<S>
    where S: TcpSocket
{
    func: Box<dyn FnOnce(HttpResponseBuilder<S>) -> HandlerResult<S> + Send>
}

impl<S> HttpMidlewareFn<S>
    where 
        S: TcpSocket
        
{
    pub fn new<F>(func: F) -> Self
        where F: Fn(HttpResponseBuilder<S>) -> HandlerResult<S> + Send,
              F: 'static
    {
        HttpMidlewareFn {
            func: Box::new(func)
        }
    }
}

#[async_trait]
impl<S> HttpMiddleware for HttpMidlewareFn<S>
    where 
        S: TcpSocket
{
    type Socket = S;

    async fn handle(self, ctx: HttpResponseBuilder<Self::Socket>) -> HandlerResult<Self::Socket> {
        (self.func)(ctx)
    }
}

pub struct HttpMidlewareFnFut<S>
    where 
        S: TcpSocket
{
    func: Box<dyn FnOnce(HttpResponseBuilder<S>) -> Pin<Box<dyn Future<Output=HandlerResult<S>> + Send>> + Send>
}

impl<S> HttpMidlewareFnFut<S>
    where 
        S: TcpSocket
{
    pub fn new<F, Fut>(func: F) -> Self
        where F: FnOnce(HttpResponseBuilder<S>) -> Fut,
              F: Send + 'static,
              Fut: Future<Output = HandlerResult<S>> + Send + 'static
    {
        Self {
            func: Box::new(|c| {
                let r = func(c);
                Box::pin(r)
            })
        }
    }
}

#[async_trait]
impl<S> HttpMiddleware for HttpMidlewareFnFut<S>
    where 
        S: TcpSocket
{
    type Socket = S;

    async fn handle(self, ctx: HttpResponseBuilder<Self::Socket>) -> HandlerResult<Self::Socket> {
        (self.func)(ctx).await
    }
}

#[derive(Debug)]
pub enum RestError {
    TcpError(TcpError)
}

impl From<TcpError> for RestError {
    fn from(v: TcpError) -> Self {
        Self::TcpError(v)
    }
}

pub enum HandlerResult<S>
    where S: TcpSocket
{
    Complete(HttpReponseComplete),
    Error(RestError),
    Pass(HttpResponseBuilder<S>)
}

impl<S> From<RestError> for HandlerResult<S>
    where S: TcpSocket
{
    fn from(v: RestError) -> Self {
        Self::Error(v)
    }
}

impl<S> From<HttpReponseComplete> for HandlerResult<S>
    where S: TcpSocket
{
    fn from(v: HttpReponseComplete) -> Self {
        Self::Complete(v)
    }
}

impl<S> From<HttpResponseBuilder<S>> for HandlerResult<S>
where S: TcpSocket
{
    fn from(v: HttpResponseBuilder<S>) -> Self {
        Self::Pass(v)
    }
}


pub async fn rest_handler<S>(ctx: HttpContext<S>)
where S: TcpSocket
{

    match ctx.request.path.as_deref() {
        Some("/") | None => {
            ctx.http_ok("text/html", "<h1>Root?</h1>").await;
        },
        _ => {
            ctx.http_reply(HttpStatusCodes::NotFound.into(), "text/html", "<h1>Not Found!</h1>").await;
        }
    }
}

pub fn allow_cors_all<S>() -> HttpMidlewareFn<S>
    where S: TcpSocket
{
    HttpMidlewareFn::new(|mut ctx: HttpResponseBuilder<S>| {
        ctx.additional_headers.push(HttpServerHeader { name: "Access-Control-Allow-Origin".into(), value: "*".into() });
        ctx.into()
    })
}

pub async fn not_found_fn<S>(ctx: HttpResponseBuilder<S>) -> HandlerResult<S>
    where S: TcpSocket
{
    let html = format!("<h1>Not found!</h1><p>Request URL: <code>{:?}</code>, method <code>{:?}</code>.</p>", ctx.ctx.request.path, ctx.ctx.request.method);
    match ctx.response(HttpStatusCodes::NotFound, "text/html".into(), Some(&html)).await {
        Ok(c) => c.into(),
        Err(e) => e.into()
    }
}

pub fn not_found<S>() -> HttpMidlewareFnFut<S>
    where S: TcpSocket
{
    HttpMidlewareFnFut::new(not_found_fn)
}
