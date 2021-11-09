use std::pin::Pin;

use async_trait::async_trait;
use futures::Future;
use crate::{HandlerResult, HandlerResultOk, middleware::{HttpMiddleware, HttpMiddlewareContext, HttpMiddlewareRunner}, response_builder::HttpResponseBuilder};

pub struct HttpMidlewareFn<C>
where
    C: HttpMiddlewareContext,
{
    func: Box<dyn FnOnce(HttpResponseBuilder<C>) -> HandlerResult<C> + Send>,
}

impl<C> HttpMidlewareFn<C>
where
    C: HttpMiddlewareContext,
{
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(HttpResponseBuilder<C>) -> HandlerResult<C> + Send,
        F: 'static,
    {
        HttpMidlewareFn {
            func: Box::new(func),
        }
    }
}

#[async_trait]
impl<C> HttpMiddleware for HttpMidlewareFn<C>
where
    C: HttpMiddlewareContext,
{
    type Context = C;

    async fn handle<N>(self, ctx: HttpResponseBuilder<Self::Context>, next: N) -> HandlerResult<Self::Context>
        where N: HttpMiddlewareRunner<Context = Self::Context> 
    {
        let res = (self.func)(ctx);
        match res {
            Ok(HandlerResultOk::Complete(c)) => {
                Ok(c.into())
            },
            Ok(HandlerResultOk::Pass(pass)) => {
                next.run(pass).await
            },
            Err(e) => {
                Err(e)
            },
        }
    }
}





pub struct HttpMidlewareFnFut<C>
where
    C: HttpMiddlewareContext,
{
    func: Box<
        dyn FnOnce(HttpResponseBuilder<C>) -> Pin<Box<dyn Future<Output = HandlerResult<C>> + Send>>
            + Send,
    >,
}

impl<C> HttpMidlewareFnFut<C>
where
    C: HttpMiddlewareContext
{
    pub fn new<F, Fut>(func: F) -> Self
    where
        F: FnOnce(HttpResponseBuilder<C>) -> Fut,
        F: Send + 'static,
        Fut: Future<Output = HandlerResult<C>> + Send + 'static,
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
impl<C> HttpMiddleware for HttpMidlewareFnFut<C>
where
    C: HttpMiddlewareContext,
{
    type Context = C;

    async fn handle<N>(self, ctx: HttpResponseBuilder<Self::Context>, next: N) -> HandlerResult<Self::Context>
        where N: HttpMiddlewareRunner<Context = Self::Context> 
    {
        let res = (self.func)(ctx).await;
        match res {
            Ok(HandlerResultOk::Complete(c)) => {
                Ok(c.into())
            },
            Ok(HandlerResultOk::Pass(pass)) => {
                next.run(pass).await
            },
            Err(e) => {
                Err(e)
            },
        }
    }
}