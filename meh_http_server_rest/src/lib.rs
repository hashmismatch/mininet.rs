pub mod openapi;
pub mod quick_rest;

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::Deref;
use std::pin::Pin;

use async_trait::async_trait;
use futures::pin_mut;
use meh_http_common::req::HttpServerHeader;
use meh_http_common::resp::{HttpResponseWriter, HttpStatusCodes};
use meh_http_common::stack::{TcpError, TcpSocket};
use meh_http_server::HttpContext;
use slog::warn;

pub struct HttpResponseBuilder<S>
where
    S: TcpSocket,
{
    pub additional_headers: Vec<HttpServerHeader>,
    pub extras: Extras,
    ctx: HttpContext<S>,
}

#[derive(Default)]
pub struct Extras {
    extras: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}
impl Extras {
    pub fn get<'a, T: 'static>(&'a mut self) -> &'a T
    where
        T: Default + Any + Send + Sync + 'static,
    {
        let ty = TypeId::of::<T>();
        let v = self.extras
            .entry(ty)
            .or_insert_with(|| Box::new(T::default()));
        (&**v as &dyn Any).downcast_ref::<T>().unwrap()
    }

    pub fn get_mut<'a, T>(&'a mut self) -> &'a mut T
    where
        T: Default + Any + Send + Sync + 'static,
    {
        let ty = TypeId::of::<T>();
        let v = self.extras
            .entry(ty)
            .or_insert_with(|| Box::new(T::default()));
        (&mut **v as &mut dyn Any).downcast_mut::<T>().unwrap()
    }
}

#[test]
fn test_extras() {
    let mut e = Extras::default();
    let v = e.get_mut::<u32>();
    assert_eq!(*v, 0);
}

impl<S> Deref for HttpResponseBuilder<S>
where
    S: TcpSocket,
{
    type Target = HttpContext<S>;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl<S> HttpResponseBuilder<S>
where
    S: TcpSocket,
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

#[async_trait]
pub trait HttpMiddleware: Send + Sized {
    type Socket: TcpSocket;

    async fn process(self, ctx: HttpContext<Self::Socket>) -> HandlerResult<Self::Socket> {
        let resp_builder = HttpResponseBuilder {
            additional_headers: vec![],
            ctx,
            extras: Extras::default(),
        };

        self.handle(resp_builder).await
    }

    async fn handle(
        self,
        mut ctx: HttpResponseBuilder<Self::Socket>,
    ) -> HandlerResult<Self::Socket>;

    fn chain<B>(self, other: B) -> HttpMidlewareChain<Self, B, Self::Socket>
    where
        B: HttpMiddleware<Socket = Self::Socket>,
    {
        HttpMidlewareChain::new_pair(self, other)
    }
}

pub struct HttpMidlewareChain<A, B, S>
where
    A: HttpMiddleware<Socket = S>,
    B: HttpMiddleware<Socket = S>,
    S: TcpSocket,
{
    a: A,
    b: B,
    _socket: PhantomData<S>,
}

impl<A, B, S> HttpMidlewareChain<A, B, S>
where
    A: HttpMiddleware<Socket = S>,
    B: HttpMiddleware<Socket = S>,
    S: TcpSocket,
{
    pub fn new_pair(a: A, b: B) -> Self {
        HttpMidlewareChain {
            a,
            b,
            _socket: PhantomData::default(),
        }
    }

    pub fn chain<C>(self, c: C) -> HttpMidlewareChain<Self, C, S>
    where
        C: HttpMiddleware<Socket = S>,
    {
        HttpMidlewareChain {
            a: self,
            b: c,
            _socket: PhantomData::default(),
        }
    }
}

/*
impl<A, S> HttpMidlewareChain<A, HttpMiddlewareNull<S>, S>
    where S: TcpSocket, A: HttpMiddleware<Socket=S>
{
    pub fn new(a: A) -> Self {
        HttpMidlewareChain {
            a,
            b: HttpMiddlewareNull(PhantomData::default()),
            _socket: PhantomData::default()
        }
    }
}
*/

#[derive(Default)]
pub struct HttpMiddlewareNull<S>(PhantomData<S>);

#[async_trait]
impl<S> HttpMiddleware for HttpMiddlewareNull<S>
where
    S: TcpSocket,
{
    type Socket = S;

    async fn handle(self, ctx: HttpResponseBuilder<S>) -> HandlerResult<S> {
        Ok(ctx.into())
    }
}

#[async_trait]
impl<A, B, S> HttpMiddleware for HttpMidlewareChain<A, B, S>
where
    A: HttpMiddleware<Socket = S>,
    B: HttpMiddleware<Socket = S>,
    S: TcpSocket,
{
    type Socket = S;

    async fn handle(self, ctx: HttpResponseBuilder<S>) -> HandlerResult<S> {
        let res_a = self.a.handle(ctx).await?;
        match res_a {
            HandlerResultOk::Pass(ctx) => self.b.handle(ctx).await,
            _ => Ok(res_a),
        }
    }
}

pub struct HttpMidlewareFn<S>
where
    S: TcpSocket,
{
    func: Box<dyn FnOnce(HttpResponseBuilder<S>) -> HandlerResult<S> + Send>,
}

impl<S> HttpMidlewareFn<S>
where
    S: TcpSocket,
{
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(HttpResponseBuilder<S>) -> HandlerResult<S> + Send,
        F: 'static,
    {
        HttpMidlewareFn {
            func: Box::new(func),
        }
    }
}

#[async_trait]
impl<S> HttpMiddleware for HttpMidlewareFn<S>
where
    S: TcpSocket,
{
    type Socket = S;

    async fn handle(self, ctx: HttpResponseBuilder<Self::Socket>) -> HandlerResult<Self::Socket> {
        (self.func)(ctx)
    }
}

pub struct HttpMidlewareFnFut<S>
where
    S: TcpSocket,
{
    func: Box<
        dyn FnOnce(HttpResponseBuilder<S>) -> Pin<Box<dyn Future<Output = HandlerResult<S>> + Send>>
            + Send,
    >,
}

impl<S> HttpMidlewareFnFut<S>
where
    S: TcpSocket,
{
    pub fn new<F, Fut>(func: F) -> Self
    where
        F: FnOnce(HttpResponseBuilder<S>) -> Fut,
        F: Send + 'static,
        Fut: Future<Output = HandlerResult<S>> + Send + 'static,
    {
        Self {
            func: Box::new(|c| {
                let r = func(c);
                Box::pin(r)
            }),
        }
    }
}

#[async_trait]
impl<S> HttpMiddleware for HttpMidlewareFnFut<S>
where
    S: TcpSocket,
{
    type Socket = S;

    async fn handle(self, ctx: HttpResponseBuilder<Self::Socket>) -> HandlerResult<Self::Socket> {
        (self.func)(ctx).await
    }
}

#[derive(Debug)]
pub enum RestError {
    TcpError(TcpError),
    Unknown
}

impl From<TcpError> for RestError {
    fn from(v: TcpError) -> Self {
        Self::TcpError(v)
    }
}

pub type HandlerResult<S> = Result<HandlerResultOk<S>, RestError>;

pub enum HandlerResultOk<S>
where
    S: TcpSocket,
{
    Complete(HttpReponseComplete),
    Pass(HttpResponseBuilder<S>),
}

impl<S> From<HttpReponseComplete> for HandlerResultOk<S>
where
    S: TcpSocket,
{
    fn from(v: HttpReponseComplete) -> Self {
        Self::Complete(v)
    }
}

impl<S> From<HttpResponseBuilder<S>> for HandlerResultOk<S>
where
    S: TcpSocket,
{
    fn from(v: HttpResponseBuilder<S>) -> Self {
        Self::Pass(v)
    }
}

pub async fn rest_handler<S>(ctx: HttpContext<S>)
where
    S: TcpSocket,
{
    match ctx.request.path.as_deref() {
        Some("/") | None => {
            ctx.http_ok("text/html", "<h1>Root?</h1>").await;
        }
        _ => {
            ctx.http_reply(
                HttpStatusCodes::NotFound.into(),
                "text/html",
                "<h1>Not Found!</h1>",
            )
            .await;
        }
    }
}

pub fn allow_cors_all<S>() -> HttpMidlewareFn<S>
where
    S: TcpSocket,
{
    HttpMidlewareFn::new(|mut ctx: HttpResponseBuilder<S>| {
        ctx.additional_headers.push(HttpServerHeader {
            name: "Access-Control-Allow-Origin".into(),
            value: "*".into(),
        });
        Ok(ctx.into())
    })
}

pub async fn not_found_fn<S>(ctx: HttpResponseBuilder<S>) -> HandlerResult<S>
where
    S: TcpSocket,
{
    let html = format!(
        "<h1>Not found!</h1><p>Request URL: <code>{:?}</code>, method <code>{:?}</code>.</p>",
        ctx.ctx.request.path, ctx.ctx.request.method
    );

    warn!(ctx.logger, "404 not found!");

    let r = ctx
        .response(HttpStatusCodes::NotFound, "text/html".into(), Some(&html))
        .await?;
    Ok(r.into())
}

pub fn not_found<S>() -> HttpMidlewareFnFut<S>
where
    S: TcpSocket,
{
    HttpMidlewareFnFut::new(not_found_fn)
}
