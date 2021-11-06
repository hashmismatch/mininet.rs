use std::{marker::PhantomData, pin::Pin, sync::Arc};

use futures::Future;
use meh_http_common::{resp::HttpStatusCodes, stack::TcpSocket};
use async_trait::async_trait;
use meh_http_server::HttpContext;
use serde_json::json;

use crate::{HandlerResult, HandlerResultOk, extras::Extras, response_builder::HttpResponseBuilder};


/*
pub struct Next<'a, S> where S: TcpSocket {
    pub(crate) next_middleware: &'a [Arc<dyn HttpMiddleware<Socket=S>>]
}
*/




#[async_trait]
pub trait HttpMiddleware: Send + Sized {
    type Socket: TcpSocket;

    async fn process(self, ctx: HttpContext<Self::Socket>) -> HandlerResult<Self::Socket> {
        let resp_builder = HttpResponseBuilder {
            additional_headers: vec![],
            ctx,
            extras: Extras::default(),
        };

        let res = self.handle(resp_builder).await;

        /*
        // wrong place for this!
        if let Err(e) = res {
            let msg = format!("{:?}", e);
            let error = json!({
                "error": msg
            });
            let body = serde_json::to_string_pretty(&error).unwrap();

            let r = ctx.response(HttpStatusCodes::BadRequest, Some("application/json".into()), Some(&body)).await?;
            return Ok(r.into());
        }
        */

        res
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