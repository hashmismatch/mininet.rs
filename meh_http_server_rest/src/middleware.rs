use std::{marker::PhantomData, ops::Add, pin::Pin, sync::Arc};

use frunk::{HCons, hlist::Plucker, prelude::HList};
use futures::Future;
use meh_http_common::{resp::HttpStatusCodes, stack::TcpSocket};
use async_trait::async_trait;
use meh_http_server::HttpContext;
use serde_json::json;

use crate::{HandlerResult, HandlerResultOk, extras::Extras, response_builder::HttpResponseBuilder};

#[async_trait]
pub trait HttpMiddlewareRunner: Send + Sized {
    type Context: HttpMiddlewareContext;

    async fn run(self, ctx: HttpResponseBuilder<Self::Context>) -> HandlerResult<Self::Context>;
}

pub async fn run_from_http<M, C>(mid: M, ctx: C, http_ctx: HttpContext< <<M as HttpMiddlewareRunner>::Context as HttpMiddlewareContext>::Socket >) 
    -> HandlerResult< C >
    where M: HttpMiddlewareRunner<Context = C>,
    C: HttpMiddlewareContext
{
    let ctx = HttpResponseBuilder {
        additional_headers: vec![],
        http_ctx: http_ctx,
        middleware_context: ctx,
        extras: Extras::default()
    };

    mid.run(ctx).await
}


pub trait HttpMiddlewareContext: Send {
    type Socket: TcpSocket;
}


pub struct DefaultContext<S> where S: TcpSocket
{
    _socket: PhantomData<S>
}

impl<S> HttpMiddlewareContext for DefaultContext<S>
where S: TcpSocket
{
    type Socket = S;
}



impl<S> DefaultContext<S>
where S: TcpSocket
{
    pub fn new() -> Self { Self { _socket: PhantomData::default() } }
}


#[async_trait]
pub trait HttpMiddleware: Send + Sized {
    type Context: HttpMiddlewareContext;

    async fn handle<N>(self, ctx: HttpResponseBuilder<Self::Context>, next: N) -> HandlerResult<Self::Context>
        where N: HttpMiddlewareRunner<Context = Self::Context>;
}




pub struct Null<C> {
    _ctx: PhantomData<C>
}

#[async_trait]
impl<C> HttpMiddlewareRunner for Null<C>
    where C: HttpMiddlewareContext
{
    type Context = C;

    async fn run(self, ctx: HttpResponseBuilder<Self::Context>) -> HandlerResult<Self::Context> {
        Ok(ctx.into())
    }
}

#[async_trait]
impl<C> HttpMiddleware for Null<C>
    where C: HttpMiddlewareContext
{
    type Context = C;

    async fn handle<N>(self, ctx: HttpResponseBuilder<Self::Context>, _next: N) -> HandlerResult<Self::Context>
        where N: HttpMiddlewareRunner<Context = Self::Context> 
    {
        Ok(ctx.into())
    }
}


impl<C> Null<C> {
    pub fn new() -> Self {
        Null { _ctx: PhantomData::default() }
    }
}
