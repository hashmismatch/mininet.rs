use meh_http_common::{req::HttpServerHeader, resp::HttpStatusCodes, stack::TcpSocket};
use slog::warn;

use crate::{HandlerResult, middleware::{HttpMidlewareFn, HttpMidlewareFnFut}, response_builder::HttpResponseBuilder};


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
        ctx.request.path, ctx.request.method
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
