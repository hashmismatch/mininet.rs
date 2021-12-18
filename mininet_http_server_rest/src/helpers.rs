use core::marker::PhantomData;
use alloc::boxed::Box;
use mininet_base::{req::HttpServerHeader, resp::HttpStatusCodes};
use slog::{warn};
use async_trait::async_trait;

use crate::{HandlerResult, HandlerResultOk, RestErrorContext, middleware::{ HttpMiddleware, HttpMiddlewareRunner, HttpMiddlewareContext}, middleware_fn::{HttpMidlewareFn}, response_builder::HttpResponseBuilder};


pub fn allow_cors_all<C>() -> HttpMidlewareFn<C>
where
    C: HttpMiddlewareContext
{
    HttpMidlewareFn::new(|mut ctx: HttpResponseBuilder<C>| {
        ctx.additional_headers.push(HttpServerHeader {
            name: "Access-Control-Allow-Origin".into(),
            value: "*".into(),
        });
        Ok(ctx.into())
    })
}

pub fn not_found<C>() -> NotFound<C>
where
    C: HttpMiddlewareContext + 'static
{
    NotFound {_ctx: PhantomData::default() }
}

pub struct NotFound<C> {
    _ctx: PhantomData<C>
}

#[async_trait]
impl<C> HttpMiddleware for NotFound<C>
    where C: HttpMiddlewareContext
{
    type Context = C;

    async fn handle<N>(self, ctx: HttpResponseBuilder<Self::Context>, next: N) -> HandlerResult<Self::Context>
        where N: HttpMiddlewareRunner<Context = Self::Context>
    {
        match next.run(ctx).await {
            Ok(HandlerResultOk::Pass(ctx)) => {
                let html = format!(
                    "<h1>Not found!</h1><p>Request URL: <code>{:?}</code>, method <code>{:?}</code>.</p>",
                    ctx.request.path, ctx.request.method
                );
            
                warn!(ctx.logger, "404 not found!");
            
                let r = ctx
                    .response(HttpStatusCodes::NotFound, "text/html".into(), Some(&html))
                    .await;
            
                match r {
                    Ok(c) => Ok(c.into()),
                    Err(e) => Err(RestErrorContext { error: e, ctx: None })
                }
            },
            r @ _ => r
        }
    }
}