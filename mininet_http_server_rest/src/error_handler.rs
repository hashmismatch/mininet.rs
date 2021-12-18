use core::marker::PhantomData;
use alloc::boxed::Box;
use crate::{HandlerResult, RestErrorContext, middleware::{HttpMiddleware, HttpMiddlewareContext, HttpMiddlewareRunner}, response_builder::HttpResponseBuilder};
use async_trait::async_trait;
use mininet_base::{resp::HttpStatusCodes};
use slog::{debug, error};

pub fn error_handler<C>() -> ErrorHandler<C> {
    ErrorHandler {
        _ctx: Default::default()
    }
}

pub struct ErrorHandler<C> {
    _ctx: PhantomData<C>
}

#[async_trait]
impl<C> HttpMiddleware for ErrorHandler<C>
    where C: HttpMiddlewareContext
{
    type Context=C;

    async fn handle<N>(self, ctx: HttpResponseBuilder<Self::Context>, next: N) -> HandlerResult<Self::Context>
        where N: HttpMiddlewareRunner<Context = Self::Context>
    {
        let logger = ctx.logger.clone();
        debug!(logger, "Error handler start.");

        let res = next.run(ctx).await;
        match res {
            Ok(_) => (),
            Err(mut e) => {
                error!(logger, "Encountered an error: {:?}", e.error);

                if let Some(ctx) = e.ctx.take() {
                    // try to render a response
                    let html = format!(
                        "<h1>Internal server error!!</h1><p>Error: {:?}</p><p>Request URL: <code>{:?}</code>, method <code>{:?}</code>.</p>",
                        e.error, ctx.request.path, ctx.request.method
                    );
                
                    let r = ctx
                        .response(HttpStatusCodes::InternalError, "text/html".into(), Some(&html))
                        .await;

                    match r {
                        Ok(_) => (),
                        Err(e) => {
                            error!(logger, "Failed to send the error response: {:?}", e);
                        }
                    }                    
                }

                return Err(RestErrorContext { error: e.error, ctx: None });
            }
        }

        res
    }
}