use meh_http_common::{req::HttpServerHeader, resp::HttpStatusCodes, stack::TcpSocket};
use slog::{debug, warn};

use crate::{HandlerResult, RestErrorContext, middleware::HttpMiddlewareContext, middleware_fn::{HttpMidlewareFn, HttpMidlewareFnFut}, response_builder::HttpResponseBuilder};


pub fn allow_cors_all<C>() -> HttpMidlewareFn<C>
where
    C: HttpMiddlewareContext
{
    HttpMidlewareFn::new(|mut ctx: HttpResponseBuilder<C>| {
        debug!(ctx.logger, "CORS");
        ctx.additional_headers.push(HttpServerHeader {
            name: "Access-Control-Allow-Origin".into(),
            value: "*".into(),
        });
        Ok(ctx.into())
    })
}

pub async fn not_found_fn<C>(ctx: HttpResponseBuilder<C>) -> HandlerResult<C>
where
    C: HttpMiddlewareContext
{
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
}

pub fn not_found<C>() -> HttpMidlewareFnFut<C>
where
    C: HttpMiddlewareContext + 'static
{
    HttpMidlewareFnFut::new(not_found_fn)
}
