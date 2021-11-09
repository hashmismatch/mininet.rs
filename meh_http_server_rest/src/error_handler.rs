use std::marker::PhantomData;
use crate::{HandlerResult, middleware::{HttpMiddleware, HttpMiddlewareContext, HttpMiddlewareRunner}, response_builder::HttpResponseBuilder};
use async_trait::async_trait;
use meh_http_common::stack::TcpSocket;
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
            Err(ref e) => {
                error!(logger, "Encountered an error: {:?}", e);
            }
        }

        res
    }

    // async fn handle<H, T>(
    //     self,
    //     mut ctx: HttpResponseBuilder<Self::Socket>,
    //     next: NextMiddleware<H, T>
    // ) -> HandlerResult<Self::Socket>    
    // where H: HttpMiddleware<Socket=Self::Socket>,
    // T: NPop + NPop<Target = H> + Send,
    // <T as NPop>::Remainder: Send
    // {
    //     todo!()
    //     /*
    //     let logger = ctx.logger.clone();
    //     debug!(logger, "Error handler start.");

    //     let res = next.process(ctx).await;
    //     match res {
    //         Ok(_) => (),
    //         Err(ref e) => {
    //             error!(logger, "Encountered an error: {:?}", e);
    //         }
    //     }

    //     res
    //     */
    // }
}